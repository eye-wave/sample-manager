import { DialogManager } from "../helpers";
import { listen } from "../invoke/invoke";
import { addShortcut } from "../shortcuts";

declare const terminal_window__: HTMLDialogElement;
declare const terminal_body__: HTMLPreElement;

function toggle() {
  if (!terminal_window__.open) DialogManager.open("terminal_window__");
  else DialogManager.close();
}

addShortcut("Toggle debug window", "t", 0b110, toggle);

listen("log", (e) => (terminal_body__.innerHTML += e));
