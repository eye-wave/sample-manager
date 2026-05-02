export type ToStr = string | number | boolean;
export type LooseInput = Omit<HTMLInputElement, "value"> & {
  value: Opt<Str>;
};

type Opt<T> = T | null;

declare global {
  interface Node {
    textContent: Opt<ToStr>;
  }
  interface HTMLButtonElement {
    textContent: Opt<ToStr>;
  }
}
