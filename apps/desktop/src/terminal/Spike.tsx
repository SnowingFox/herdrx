// Phase 0 terminal spike: N panes running a stress command, with parsed MiB/s.
// "flood" makes pane 0 a `cat` echo probe and measures keystroke round trips
// while the other panes flood. HERDR_SPIKE="<panes>:<mode>" runs it unattended.
import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { TerminalPane, type Stats } from "./TerminalPane";

const FLOOD = ["sh", "-c", "yes 'herdr spike line with some width to it 0123456789' | head -n 2000000; echo done"];
const STRESS: Record<string, string[] | undefined> = {
  shell: undefined,
  yes: FLOOD,
  seq: ["sh", "-c", "seq 1 3000000; echo done"],
  color: ["sh", "-c", "i=0; while [ $i -lt 200000 ]; do printf '\\033[3%dm%06d \\033[1;4%dmcolor\\033[0m wide 漢字 ✓\\n' $((i%8)) $i $((i%8)); i=$((i+1)); done; echo done"],
  flood: FLOOD,
};

const argvFor = (mode: string, i: number) => (mode === "flood" && i === 0 ? ["cat"] : STRESS[mode]);
const fresh = (): Stats => ({ parsed: 0, ended: false });
const mib = (n: number) => (n / 1048576).toFixed(1);
const pct = (xs: number[], p: number) => (xs.length ? xs.slice().sort((a, b) => a - b)[Math.min(xs.length - 1, Math.floor(xs.length * p))].toFixed(1) : "-");

export function Spike() {
  const [count, setCount] = useState(1);
  const [mode, setMode] = useState("shell");
  const [run, setRun] = useState(0);
  const [auto, setAuto] = useState(false);
  const stats = useRef<Stats[]>([]);
  const [rate, setRate] = useState("");

  useEffect(() => {
    invoke<string | null>("spike_config").then((c) => {
      if (!c) return;
      const [n, m] = c.split(":");
      setCount(+n);
      setMode(m);
      setAuto(true);
      setRun((r) => r + 1);
    });
  }, []);

  if (stats.current.length !== count) stats.current = Array.from({ length: count }, fresh);

  useEffect(() => {
    const s = stats.current;
    const started = performance.now();
    const latencies: number[] = [];
    let last = 0;
    let probe: ReturnType<typeof setInterval> | undefined;
    let waitingSince: number | undefined;

    if (mode === "flood") {
      // Each keystroke waits for its echo before the next one goes out.
      s[0].onBytes = (b) => {
        if (waitingSince !== undefined && b.includes(120 /* x */)) {
          latencies.push(performance.now() - waitingSince);
          waitingSince = undefined;
        }
      };
      probe = setInterval(() => {
        if (waitingSince === undefined && s[0].send) {
          waitingSince = performance.now();
          s[0].send("x");
        }
      }, 50);
    }

    const tick = setInterval(() => {
      const total = s.reduce((a, x) => a + x.parsed, 0);
      const secs = (performance.now() - started) / 1000;
      const line = `panes=${count} mode=${mode} now=${mib(total - last)}MiB/s total=${mib(total)}MiB avg=${mib(total / secs)}MiB/s` + (mode === "flood" ? ` echo_ms p50=${pct(latencies, 0.5)} p95=${pct(latencies, 0.95)} max=${pct(latencies, 1)} n=${latencies.length}` : "");
      const chunks = s.reduce((a, x) => a + (x.chunks ?? 0), 0);
      const writeMs = s.reduce((a, x) => a + (x.writeMs ?? 0), 0);
      const extra = ` chunks=${chunks} avg_chunk=${chunks ? (total / chunks / 1024).toFixed(1) : 0}KiB parse_share=${((writeMs / 1000 / secs) * 100).toFixed(0)}%`;
      setRate(line);
      last = total;
      const flooders = mode === "flood" ? s.slice(1) : s;
      const done = mode !== "shell" && flooders.length > 0 && flooders.every((x) => x.ended);
      if (auto && (done || secs > 120)) invoke("spike_report", { line: `${line}${extra} secs=${secs.toFixed(1)} finished=${done}`, done: true });
    }, 1000);

    return () => {
      clearInterval(tick);
      if (probe) clearInterval(probe);
    };
  }, [run, auto]);

  const cols = Math.ceil(Math.sqrt(count));

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center gap-3 border-b border-neutral-800 px-3 py-2">
        <label>
          panes{" "}
          <select value={count} onChange={(e) => setCount(+e.target.value)} className="bg-neutral-800">
            {[1, 2, 4, 6, 9].map((n) => (
              <option key={n}>{n}</option>
            ))}
          </select>
        </label>
        <label>
          run{" "}
          <select value={mode} onChange={(e) => setMode(e.target.value)} className="bg-neutral-800">
            {Object.keys(STRESS).map((k) => (
              <option key={k}>{k}</option>
            ))}
          </select>
        </label>
        <button className="rounded bg-neutral-700 px-2" onClick={() => { stats.current = []; setRun(run + 1); }}>
          restart
        </button>
        <span className="ml-auto font-mono text-xs text-neutral-400">{rate}</span>
      </div>
      <div key={`${run}-${count}-${mode}`} className="grid min-h-0 flex-1 gap-1 p-1" style={{ gridTemplateColumns: `repeat(${cols}, 1fr)` }}>
        {stats.current.map((s, i) => (
          <TerminalPane key={i} argv={argvFor(mode, i)} stats={s} />
        ))}
      </div>
    </div>
  );
}
