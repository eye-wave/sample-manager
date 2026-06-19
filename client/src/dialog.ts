import { w } from "./alias";
import { updateCurrentTheme } from "./helpers";
import { addShortcut } from "./shortcuts";
import type { Writable } from "./store";

export type DialogManager = {
  open(id: string): void;
  close(): void;
};

export const createDialogManager = (activeDialog: Writable<string | null>) => {
  const ids: string[] = [];

  addShortcut("Close settings dialog", "Escape", 0, () => {
    if (conf_dial__.open) updateCurrentTheme();
    close();
  });

  const dial = (id: string | null = activeDialog.get()): HTMLDialogElement | null =>
    // @ts-expect-error
    id ? (w?.[id] as HTMLDialogElement) : null;

  function addId(id: string) {
    if (ids.includes(id)) return;
    ids.push(id);
    const d = dial(id);
    if (!d) return;
    d.onclose = close;
  }

  function close() {
    for (const id of ids) {
      const d = dial(id);
      if (!d) continue;
      d.close();
      d.blur();
    }
    activeDialog.set(null);
  }

  function open(id: string) {
    if (activeDialog.get() !== null) return false;
    // @ts-expect-error
    document.activeElement?.blur();

    const d = dial(id);
    if (!d) return false;

    activeDialog.set(id);
    addId(id);

    !d.open && d.showModal();
    return d.open;
  }

  return { open, close };
};
