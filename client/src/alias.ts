export const d = document;
export const txt = (str: string = "") => d.createTextNode(str);
export const $el = (el: string) => d.createElement(el);

export const w = window;

export const KEYDOWN = "keydown" as const;
export const ONCLICK = "onclick" as const;
export const BEFOREEND = "beforeend" as const;
export const APPEND_CHILD = "appendChild" as const;
export const QUERY_SELECTOR = "querySelector" as const;
export const ADD_EVENT_LISTENER = "addEventListener" as const;
export const INSERT_ADJACENT_HTML = "insertAdjacentHTML" as const;
