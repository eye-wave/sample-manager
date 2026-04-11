(() => {
  const pending = new Map();
  let nextId = 1;

  const inv = (id, payload) => {
    return new Promise((resolve, reject) => {
      const callId = nextId++;
      pending.set(callId, { resolve, reject });

      window.ipc.postMessage(`${id}:${callId}:${payload ?? ""}`);
    });
  };

  function res(callId, response) {
    const pendingCall = pending.get(callId);
    if (pendingCall) {
      pendingCall.resolve(response);
      pending.delete(callId);

      if (pending.size < 1) {
        nextId = 0;
      }
    }
  }

  window.invoke = inv;
  window._r = res;

  const toLog = (args) =>
    args.reduce((str, n) => str + `${n} `, "").slice(0, -1);

  console.log = (...args) => inv("log", "L" + toLog(args));
  console.warn = (...args) => inv("log", "W" + toLog(args));
  console.error = (...args) => inv("log", "E" + toLog(args));
})();
