import { addShortcut } from "../shortcuts";
import type { DialogManager } from "../dialog";
import { onLog } from "@state";

const MAX_LOG_LINES = 200;

export const createTerminal = (dialogs: DialogManager) => {
  addShortcut("Toggle debug window", "t", 0b110, () => dialogs.open("terminal_window__"));

  onLog.on((htmlPayload) => {
    const row = document.createElement("div");
    row.innerHTML = htmlPayload;

    terminal_body__.appendChild(row);

    while (terminal_body__.children.length > MAX_LOG_LINES) {
      terminal_body__.firstElementChild?.remove();
    }

    terminal_body__.scrollTop = terminal_body__.scrollHeight;
  });

  terminal_close__.onclick = dialogs.close;
};
