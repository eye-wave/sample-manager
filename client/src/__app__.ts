import { w } from "./alias";
import * as IPC from "./gen/ipc-gen";
import { updateCurrentTheme } from "./helpers";
import { invoke } from "./invoke/invoke";

/// BUILD start
w.oncontextmenu = (e) => e.preventDefault();
/// BUILD end

declare const titlebar_handle__: HTMLDivElement;
declare const btn_minimize__: HTMLButtonElement;
declare const btn_maximize__: HTMLButtonElement;
declare const btn_closeWindow__: HTMLButtonElement;

titlebar_handle__.onmousedown = () => invoke(IPC.DRAG_WINDOW);

btn_minimize__.onclick = () => invoke(IPC.MINIMIZE_WINDOW);
btn_maximize__.onclick = () => invoke(IPC.MAXIMIZE_WINDOW);
btn_closeWindow__.onclick = () => invoke(IPC.CLOSE_WINDOW);

/// DEV start
updateCurrentTheme();
/// DEV end

w.addEventListener(
  "wheel",
  (e) => {
    if (e.ctrlKey) e.preventDefault();
  },
  { passive: false },
);
