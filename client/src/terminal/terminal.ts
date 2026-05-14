import { listen } from "../invoke/invoke";
import { addShortcut } from "../shortcuts";

declare const terminal_window__: HTMLDivElement;
declare const conf_dial__: HTMLDialogElement;

let isOpen = false;

function toggle() {
  if (conf_dial__.open) return;

  isOpen = !isOpen;
  terminal_window__.style.display = isOpen ? "" : "none";
}

addShortcut("Toggle debug window", "t", 0b110, toggle);

listen("log", (e) => (terminal_window__.innerHTML += e));
