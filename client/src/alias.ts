export const d = document;
export const txt = (str: string = "") => d.createTextNode(str);
export const $el = (el: keyof HTMLElementTagNameMap) => d.createElement(el);

export const w = window;
