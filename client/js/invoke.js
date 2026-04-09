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
})();
