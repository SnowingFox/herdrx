// One core terminal drawn by ghostty-web. Credit goes back only after
// ghostty-web has parsed a chunk, so a busy program can't outrun the page.
import { useEffect, useRef } from "react";
import { FitAddon, Terminal } from "ghostty-web";
import * as core from "./core";

export interface Stats {
  /** Bytes ghostty-web has finished parsing. */
  parsed: number;
  ended: boolean;
  chunks?: number;
  writeMs?: number;
  /** Set once attached: types into the program. */
  send?: (text: string) => void;
  /** Called with each chunk as it arrives (the latency probe watches it). */
  onBytes?: (bytes: Uint8Array) => void;
}

export function TerminalPane({ argv, stats }: { argv?: string[]; stats: Stats }) {
  const host = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const term = new Terminal({
      fontSize: 14,
      fontFamily: "JetBrains Mono, Menlo, monospace",
      theme: { background: "#161616", foreground: "#d4d4d4" },
    });
    const fit = new FitAddon();
    term.loadAddon(fit);
    term.open(host.current!);
    fit.fit();

    let channel: number | undefined;
    let gone = false;
    let owed = 0;
    // Bytes can arrive before term_open resolves: they stay owed until the
    // channel is known, or the core's window shrinks for good.
    const payCredit = () => {
      if (channel === undefined || owed === 0) return;
      core.credit(channel, owed);
      owed = 0;
    };
    const disposers: { dispose(): void }[] = [];

    core
      .open(
        term.cols,
        term.rows,
        argv,
        (bytes) => {
          stats.onBytes?.(bytes);
          // ghostty-web parses inside write() and only defers its callback to
          // the next frame, so the bytes are consumed when it returns. Credit
          // then, once per task: waiting for the frame capped a pane at the
          // core's window 60 times a second.
          const t0 = performance.now();
          term.write(bytes);
          stats.writeMs = (stats.writeMs ?? 0) + performance.now() - t0;
          stats.chunks = (stats.chunks ?? 0) + 1;
          stats.parsed += bytes.length;
          if (owed === 0) queueMicrotask(payCredit);
          owed += bytes.length;
        },
        (e) => {
          stats.ended = true;
          term.write(e.kind === "lost" ? "\r\n[lost the core]\r\n" : "\r\n[ended]\r\n");
        },
      )
      .then((o) => {
        if (gone) return core.close(o.channel);
        channel = o.channel;
        payCredit();
        stats.send = (text) => core.input(o.channel, text);
        disposers.push(term.onData((d) => core.input(o.channel, d)));
        disposers.push(term.onResize(({ cols, rows }) => core.resize(o.channel, cols, rows)));
      })
      .catch((e) => {
        stats.ended = true;
        term.write(`\r\n[${e}]\r\n`);
      });

    const el = host.current!;
    const observer = new ResizeObserver(() => fit.fit());
    observer.observe(el);
    const onFocus = () => channel !== undefined && core.focus(channel, true);
    const onBlur = () => channel !== undefined && core.focus(channel, false);
    el.addEventListener("focusin", onFocus);
    el.addEventListener("focusout", onBlur);

    return () => {
      gone = true;
      observer.disconnect();
      el.removeEventListener("focusin", onFocus);
      el.removeEventListener("focusout", onBlur);
      disposers.forEach((d) => d.dispose());
      if (channel !== undefined) core.close(channel);
      term.dispose();
    };
  }, []);

  return <div ref={host} className="h-full w-full overflow-hidden" />;
}
