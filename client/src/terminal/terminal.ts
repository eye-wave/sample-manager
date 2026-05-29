import { DialogManager } from "../dialog";
import { listen } from "../invoke/invoke";
import { addShortcut } from "../shortcuts";

addShortcut("Toggle debug window", "t", 0b110, () => DialogManager.open("terminal_window__"));

const MAX_LOG_LINES = 200;

listen("log", (htmlPayload) => {
  const row = document.createElement("div");
  row.innerHTML = htmlPayload;

  terminal_body__.appendChild(row);

  while (terminal_body__.children.length > MAX_LOG_LINES) {
    terminal_body__.firstElementChild?.remove();
  }

  terminal_body__.scrollTop = terminal_body__.scrollHeight;
});

terminal_close__.onclick = DialogManager.close;
