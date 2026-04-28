import { w } from "../alias";
import * as IPC from "../gen/ipc-gen";

type PromiseReturn = {
  resolve: (value: string | PromiseLike<string>) => void;
  // biome-ignore lint/suspicious/noExplicitAny: this is just reject type
  reject: (reason?: any) => void;
};

const pending = new Map<number, PromiseReturn>();
let nextId = 1;

export const invoke = (id: number, payload?: string) => {
  return new Promise<string>((resolve, reject) => {
    const callId = nextId++;
    pending.set(callId, { resolve, reject });
    /// DEV start
    if (typeof ipc === "undefined")
      return console.warn(`Trying to call IPC command: "${id}" in the browser window.`);
    /// DEV end
    ipc.postMessage(`${id}:${callId}:${payload ?? ""}`);
  });
};

function res(callId: number, response: string) {
  const pendingCall = pending.get(callId);
  if (pendingCall) {
    pendingCall.resolve(response);
    pending.delete(callId);

    if (pending.size < 1) {
      nextId = 0;
    }
  }
}

type CB = (payload: string) => void;

const listeners = new Map<string, Set<CB>>();

function send(id: string, payload: string) {
  const set = listeners.get(id);
  if (!set) return;

  for (const cb of set) {
    cb(payload);
  }
}

export function listen(id: string, callback: CB) {
  if (!listeners.has(id)) {
    listeners.set(id, new Set());
  }
  listeners.get(id)?.add(callback);
}

w._r = res;
w._s = send;

/// DEV start
if (typeof ipc !== "undefined") {
  const toStr = <T>(i: T) => {
    const type = typeof i;
    if (type === "string") return i;
    if (type === "number") return `${i}`;
    if (type === "boolean") return i ? "true" : "false";
    if (type === "object") {
      const name = i?.constructor.name;
      const json = JSON.stringify(i, null, 2);
      const jsonHidden = json === "{}" ? "" : json;

      return `\x1b[90m[${name}]\x1b[0m${jsonHidden}`;
    }
  };

  const toLog = (args: unknown[]) =>
    args.reduce((str: string, n: unknown) => str + toStr(n) + " ", "").slice(0, -1);

  console.log = (...args) => invoke(IPC.LOG, "L" + toLog(args));
  console.warn = (...args) => invoke(IPC.LOG, "W" + toLog(args));
  console.error = (...args) => invoke(IPC.LOG, "E" + toLog(args));
}
/// DEV end
