// The backend's terminal commands (src-tauri/src/term.rs).
import { Channel, invoke } from "@tauri-apps/api/core";

export type Ended = { kind: "closed"; reason: unknown } | { kind: "lost" };

export interface Opened {
  channel: number;
  terminal: unknown;
}

export function open(cols: number, rows: number, argv: string[] | undefined, onData: (bytes: Uint8Array) => void, onEnd: (e: Ended) => void) {
  const data = new Channel<ArrayBuffer>();
  data.onmessage = (b) => onData(new Uint8Array(b));
  const end = new Channel<Ended>();
  end.onmessage = onEnd;
  return invoke<Opened>("term_open", { argv, cols, rows, onData: data, onEnd: end });
}

export const input = (channel: number, data: string) => invoke("term_input", { channel, data });
export const credit = (channel: number, n: number) => invoke("term_credit", { channel, n });
export const resize = (channel: number, cols: number, rows: number) => invoke("term_resize", { channel, cols, rows });
export const focus = (channel: number, focused: boolean) => invoke("term_focus", { channel, focused });
export const close = (channel: number) => invoke("term_close", { channel });
