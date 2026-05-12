import { d, w } from "./alias";
import * as IPC from "./gen/ipc-gen";
import { updateCurrentTheme } from "./helpers";
import { invoke } from "./invoke/invoke";

/// BUILD start
w.oncontextmenu = (e) => e.preventDefault();
/// BUILD end

declare const btn_minimize__: HTMLButtonElement;
declare const btn_maximize__: HTMLButtonElement;
declare const btn_closeWindow__: HTMLButtonElement;

w.onmousedown = (e) => {
  if (e.clientY > 20) return;
  if (e.target === btn_minimize__) return;
  if (e.target === btn_maximize__) return;
  if (e.target === btn_closeWindow__) return;

  invoke(IPC.DRAG_WINDOW);
};

btn_minimize__.onclick = () => invoke(IPC.MINIMIZE_WINDOW);
btn_maximize__.onclick = () => invoke(IPC.MAXIMIZE_WINDOW);
btn_closeWindow__.onclick = () => invoke(IPC.CLOSE_WINDOW);

/// DEV start
updateCurrentTheme();

if (typeof ipc === "undefined") {
  import("../styles/theme.scss");
}

window.addEventListener("error", (event) => {
  const err = event.error;

  console.error(
    "Caught error:",
    [
      err?.message || event.message,
      err?.stack,
      event.filename,
      event.lineno,
      event.colno,
      event.type,
    ].join(" "),
  );
});

window.addEventListener("unhandledrejection", (event) => {
  const reason = event.reason;

  console.error(
    "Unhandled rejection:",
    [reason?.message ?? String(reason), reason?.stack, reason?.name].join(" "),
  );
});

/// DEV end

w.addEventListener(
  "wheel",
  (e) => {
    if (e.ctrlKey) e.preventDefault();
  },
  { passive: false },
);

(() => {
  const EDGE = 6;

  const SouthEast = 0;
  const NorthEast = 1;
  const SouthWest = 2;
  const NorthWest = 3;
  const East = 4;
  const West = 5;
  const North = 6;
  const South = 7;

  const HITS = ["nwse", "nesw", "nesw", "nwse", "ew", "ew", "ns", "ns"];

  function getDir(x: number, y: number) {
    const w = window.innerWidth,
      h = window.innerHeight;
    const r = x >= w - EDGE,
      b = y >= h - EDGE;
    const l = x <= EDGE,
      t = y <= EDGE;
    if (r && b) return SouthEast;
    if (r && t) return NorthEast;
    if (l && b) return SouthWest;
    if (l && t) return NorthWest;
    if (r) return East;
    if (l) return West;
    if (t) return North;
    if (b) return South;
    return null;
  }
  d.addEventListener("mousemove", (e) => {
    const hit = getDir(e.clientX, e.clientY);
    d.documentElement.style.cursor = hit ? HITS[hit] + "-resize" : "";
  });
  d.addEventListener("mousedown", (e) => {
    if (e.button !== 0) return;
    const hit = getDir(e.clientX, e.clientY);
    if (hit) {
      e.preventDefault();
      ipc.postMessage("0:" + hit);
    }
  });
})();
