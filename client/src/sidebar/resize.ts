import { d, w } from "../alias";
import { invoke, IPC } from "../invoke/invoke";

const MIN_WIDTH = 160;
const MAX_WIDTH = 480;

let startX = 0;
let startWidth = 0;
let width = 280;

export async function initSidebarResize() {
  const sidebar = sidebar_container__;
  const handle = sidebar_resize__;

  const saved = +(await invoke(IPC.GET_CONFIG_FIELD, `sidebar_width`));
  if (saved) sidebar.style.width = `${saved}px`;

  const onMove = (e: MouseEvent) => {
    const delta = e.clientX - startX;
    resizeHandle(startWidth + delta);
  };

  const onUp = async (_: MouseEvent) => {
    handle.classList.remove("dragging");
    d.body.style.cursor = "";
    d.body.style.userSelect = "";

    await invoke(IPC.PATCH_CONFIG, `{"sidebar_width":${width}}`);

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

export function resizeHandle(size: number) {
  width = Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, size));
  sidebar_container__.style.width = `${width}px`;
}
