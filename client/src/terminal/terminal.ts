import { DialogManager } from "../dialog";
import { listen } from "../invoke/invoke";
import { addShortcut } from "../shortcuts";

function toggle() {
  if (!DialogManager.open("terminal_window__")) DialogManager.close();
}

addShortcut("Toggle debug window", "t", 0b110, toggle);

listen("log", (e) => (terminal_body__.innerHTML += e));
