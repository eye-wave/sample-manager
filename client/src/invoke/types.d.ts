type CB = (payload: string) => void;

declare global {
  function invoke(fn_name: string, payload?: string): Promise<string>;
  function listen(id: string, callback: CB): void;

  function _r(id: number, payload: string): void;
  function _s(id: string, payload: string): void;

  const ipc: {
    postMessage(message: string): void;
  };

  interface Window {
    invoke: typeof invoke;
    listen: typeof listen;
    _r: typeof _r;
    _s: typeof _s;
    ipc: typeof ipc;
  }
}

export {};
