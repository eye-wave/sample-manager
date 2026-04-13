type CB = (payload: string) => void;

declare global {
  function invoke(fn_name: string, payload?: string): Promise<string>;
  function listen(id: string, callback: CB);

  function _r(id: number, payload: string): void;
  function _s(id: string, payload: string): void;

  interface Window {
    invoke: invoke;
    listen: listen;
    _r: _r;
    _s: _s;
  }
}

export {};
