import { w } from "./alias";
import { isFocusElement } from "./helpers";

type Callback = (e: KeyboardEvent) => void;

const shortcuts = new Map<string, Callback>();
const shortcutMeta = new Map<string, string>();

w.addEventListener("keydown", (e) => {
  if (isFocusElement(e.target)) return;

  const key = e.key === " " ? "Space" : e.key;

  const bitmask = (+e.ctrlKey << 2) | (+e.shiftKey << 1) | +e.altKey;
  const query = bitmask + key.toUpperCase();

  shortcuts.get(query)?.(e);
});

/**
 * @param bitmask {number}
 *  1st bit - ctrlKey
 *
 *  2nd bit - shiftKey
 *
 *  3rd bit - altKey
 *
 */
export function addShortcut(
  description: string,
  key: string,
  bitmask: number,
  callback: Callback,
) {
  const k = bitmask + (key === " " ? "Space" : key).toUpperCase();

  if (shortcuts.has(k)) return;
  shortcuts.set(k, callback);
  shortcutMeta.set(k, description);
}

export function* iterateShortcuts() {
  for (const [key, value] of shortcutMeta) {
    yield [key, value];
  }
}
