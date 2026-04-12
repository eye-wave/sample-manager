declare global {
  function invoke(fn_name: string, payload?: string): Promise<string>;
  function _r(id: number, payload: string): void;

  interface Window {
    invoke: invoke;
    _r: _r;
  }
}

export {};
