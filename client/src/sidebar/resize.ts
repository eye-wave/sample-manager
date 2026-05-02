import { d, w } from "../alias";
import * as IPC from "../gen/ipc-gen";
import { invoke } from "../invoke/invoke";

const MIN_WIDTH = 160;
const MAX_WIDTH = 480;

declare const sidebar_resize__: HTMLDivElement;
declare const sidebar_container__: HTMLDivElement;

export async function initSidebarResize() {
  const sidebar = sidebar_container__;
  const handle = sidebar_resize__;

  const saved = +(await invoke(IPC.GET_CONFIG_FIELD, `sidebarWidth`));
  if (saved) sidebar.style.width = `${saved}px`;

  let startX = 0;
  let startWidth = 0;
  let width = saved;

  const onMove = (e: MouseEvent) => {
    const delta = e.clientX - startX;
    width = Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, startWidth + delta));
    sidebar.style.width = `${width}px`;
  };

  const onUp = async (_: MouseEvent) => {
    handle.classList.remove("dragging");
    d.body.style.cursor = "";
    d.body.style.userSelect = "";

    await invoke(IPC.PATCH_CONFIG, `{"sidebarWidth":${width}}`);

    w.removeEventListener("mousemove", onMove);
    w.removeEventListener("mouseup", onUp);
  };

  handle.onmousedown = (e) => {
    e.preventDefault();
    startX = e.clientX;
    startWidth = sidebar.offsetWidth;

    handle.classList.add("dragging");

    d.body.style.cursor = "col-resize";
    d.body.style.userSelect = "none";

    w.addEventListener("mousemove", onMove);
    w.addEventListener("mouseup", onUp);
  };
}
