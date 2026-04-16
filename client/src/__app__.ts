/// BUILD start
window.oncontextmenu = (e) => e.preventDefault();
/// BUILD end

declare const titlebar_handle: HTMLDivElement;
declare const btn_minimize: HTMLButtonElement;
declare const btn_maximize: HTMLButtonElement;
declare const btn_closeWindow: HTMLButtonElement;

titlebar_handle.onmousedown = () => invoke("drag_window");

btn_minimize.onclick = () => invoke("minimize_window");
btn_maximize.onclick = () => invoke("maximize_window");
btn_closeWindow.onclick = () => invoke("close_window");
