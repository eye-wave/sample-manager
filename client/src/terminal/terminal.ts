import { DialogManager } from "../dialog";
import { listen } from "../invoke/invoke";
import { addShortcut } from "../shortcuts";

addShortcut("Toggle debug window", "t", 0b110, () => DialogManager.open("terminal_window__"));
listen("log", (e) => (terminal_body__.innerHTML += e));

terminal_close__.onclick = DialogManager.close;
