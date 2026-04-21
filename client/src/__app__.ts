import { ONCLICK, w } from "./alias";

/// BUILD start
w.oncontextmenu = (e) => e.preventDefault();
/// BUILD end

declare const titlebar_handle: HTMLDivElement;
declare const btn_minimize: HTMLButtonElement;
declare const btn_maximize: HTMLButtonElement;
declare const btn_closeWindow: HTMLButtonElement;

titlebar_handle.onmousedown = () => invoke("drag_window");

btn_minimize[ONCLICK] = () => invoke("minimize_window");
btn_maximize[ONCLICK] = () => invoke("maximize_window");
btn_closeWindow[ONCLICK] = () => invoke("close_window");
