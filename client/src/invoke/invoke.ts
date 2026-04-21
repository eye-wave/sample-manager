import { w } from "../alias";

(() => {
  const pending = new Map();
  let nextId = 1;
  const inv = (id: string, payload?: string) => {
    return new Promise<string>((resolve, reject) => {
      const callId = nextId++;
      pending.set(callId, { resolve, reject });
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

  function listen(id: string, callback: CB) {
    if (!listeners.has(id)) {
      listeners.set(id, new Set());
    }
    listeners.get(id)?.add(callback);
  }

  w.invoke = inv;
  w.listen = listen;

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

    console.log = (...args) => inv("log", "L" + toLog(args));
    console.warn = (...args) => inv("log", "W" + toLog(args));
    console.error = (...args) => inv("log", "E" + toLog(args));
  }
  /// DEV end
})();
