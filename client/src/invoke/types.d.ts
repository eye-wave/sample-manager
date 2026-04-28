declare global {
  function _r(id: number, payload: string): void;
  function _s(id: string, payload: string): void;

  const ipc: {
    postMessage(message: string): void;
  };

  interface Window {
    _r: typeof _r;
    _s: typeof _s;
    ipc: typeof ipc;
  }
}

export {};
