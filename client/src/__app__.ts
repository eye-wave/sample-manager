import { w } from "./alias";
import { updateCurrentTheme } from "./helpers";

/// BUILD start
w.oncontextmenu = (e) => e.preventDefault();
/// BUILD end

declare const titlebar_handle__: HTMLDivElement;
declare const btn_minimize__: HTMLButtonElement;
declare const btn_maximize__: HTMLButtonElement;
declare const btn_closeWindow__: HTMLButtonElement;

titlebar_handle__.onmousedown = () => invoke("drag_window");

btn_minimize__.onclick = () => invoke("minimize_window");
btn_maximize__.onclick = () => invoke("maximize_window");
btn_closeWindow__.onclick = () => invoke("close_window");

/// DEV start
updateCurrentTheme();
/// DEV end
