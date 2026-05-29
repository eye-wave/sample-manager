import { w } from "./alias";
import { updateCurrentTheme } from "./helpers";
import { addShortcut } from "./shortcuts";

export const DialogManager = (() => {
  const ids: string[] = [];

  let current: null | string = null;

  addShortcut("Close settings dialog", "Escape", 0, () => {
    if (conf_dial__.open) {
      updateCurrentTheme();
    }

    DialogManager.close();
  });

  // @ts-expect-error
  const dial = (id = current): HTMLDialogElement | null => w?.[id] as HTMLDialogElement;

  function addId(id: string) {
    if (ids.includes(id)) return;
    ids.push(id);

    const d = dial(id);
    if (!d) return;

    d.onclose = DialogManager.close;
  }

  return {
    close() {
      ids.forEach((id) => {
        const d = dial(id);
        if (!d) return;

        d.close();
        d.blur();
      });

      current = null;
    },
    open(id: string) {
      if (current !== null) return false;

      const d = dial(id);
      if (!d) return;

      current = id;
      addId(id);

      !d.open && d.showModal();
      return d.open;
    },
  };
})();
