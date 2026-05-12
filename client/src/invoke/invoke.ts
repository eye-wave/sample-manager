import { w } from "../alias";
import * as IPC from "../gen/ipc-gen";

type PromiseReturn = {
  resolve: (value: string | PromiseLike<string>) => void;
  // biome-ignore lint/suspicious/noExplicitAny: this is just reject type
  reject: (reason?: any) => void;
};

const pending = new Map<number, PromiseReturn>();
let nextId = 1;

export const invoke = <T>(id: number, payload?: T) => {
  return new Promise<string>((resolve, reject) => {
    const callId = nextId++;
    pending.set(callId, { resolve, reject });
    /// DEV start

    if (typeof ipc === "undefined") {
      const key = Object.entries(IPC).find((e) => e[1] === id)?.[0];
      return console.warn(`Trying to call IPC command: "${key}" in the browser window.`);
    }
    /// DEV end

    let data = "";
    if (typeof payload === "object") data = JSON.stringify(payload);
    else data = payload as string;

    ipc.postMessage(`${id}:${callId}:${data ?? ""}`);
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
  const inspect = (value: unknown, depth = 0, maxDepth = 3): string => {
    const indent = "  ".repeat(depth);

    if (value === null) return "null";
    if (value === undefined) return "undefined";

    const type = typeof value;

    if (type !== "object") {
      if (type === "string") return `"${value}"`;
      return String(value);
    }

    const obj = value as Record<string, unknown>;

    const name =
      value instanceof Object && value.constructor ? value.constructor.name : "Object";

    if (depth >= maxDepth) {
      return `[\x1b[90m${name}\x1b[0m] {...}`;
    }

    if (Array.isArray(value)) {
      const len = value.length;

      const items: string[] = [];

      for (let i = 0; i < Math.min(len, 10); i++) {
        items.push(`${i}: ${inspect(value[i] as unknown, depth + 1, maxDepth)}`);
      }

      const more = len > 10 ? ` ... +${len - 10}` : "";

      return `[\x1b[90m${name}\x1b[0m:${len}] {\n${indent}  ${items.join(
        "\n" + indent + "  ",
      )}${more ? "\n" + indent + more : ""}\n${indent}}`;
    }

    const entries: string[] = [];

    try {
      for (const k of Object.keys(obj)) {
        const v = obj[k];
        entries.push(`${k}: ${inspect(v, depth + 1, maxDepth)}`);
      }
    } catch {
      return `[\x1b[90m${name}\x1b[0m] <uninspectable>`;
    }

    if (entries.length === 0) {
      return `[\x1b[90m${name}\x1b[0m] {}`;
    }

    return `[\x1b[90m${name}\x1b[0m] {\n${indent}  ${entries.join(
      "\n" + indent + "  ",
    )}\n${indent}}`;
  };

  const toLog = (args: unknown[]) => args.map((a) => inspect(a)).join(" ");

  console.log = (...args) => invoke(IPC.LOG, "L" + toLog(args));
  console.warn = (...args) => invoke(IPC.LOG, "W" + toLog(args));
  console.error = (...args) => invoke(IPC.LOG, "E" + toLog(args));
}
/// DEV end
