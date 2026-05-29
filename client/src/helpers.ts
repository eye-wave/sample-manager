import { d } from "./alias";
import { invoke, IPC } from "./invoke/invoke";

export function escapeHTML(str: string) {
  return str
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#039;");
}

export function renderTags<E extends HTMLElement>(tagsEl: E, tags: string[]) {
  tagsEl.innerHTML = tags
    .filter((t) => t)
    .map((t) => /* HTML */ `<span class="tag">${t}</span>`)
    .join("");
}

const SEPARATOR =
  /// UNIX start
  "/";
/// UNIX end
/// WIN start
("\\\\");
/// WIN end

const basenameRegex = new RegExp(`[${SEPARATOR}]`);

export function basename(name: string) {
  return name.split(basenameRegex).pop() ?? name;
}

export function joinPath(...parts: string[]) {
  const len = parts.length;
  if (len === 0) return "";

  let out = "";

  for (let i = 0; i < len; i++) {
    const part = parts[i];
    if (!part) continue;

    const isFirst = out === "";

    let start = 0;
    if (!isFirst) {
      while (start < part.length && part[start] === SEPARATOR) start++;
    }

    let end = part.length;
    while (end > start && part[end - 1] === SEPARATOR) end--;

    if (end <= start) continue;
    if (!isFirst) out += SEPARATOR;

    out += part.slice(start, end);
  }

  return out;
}

/// DEV start
const devStyle = document.createElement("style");
document.head.append(devStyle);
/// DEV end

export function updateThemeCss(css: string) {
  /// DEV start
  devStyle.innerHTML = css;
  /// DEV end
  /// BUILD start
  // biome-ignore lint/style/noNonNullAssertion: trust
  d.querySelector("style")!.innerHTML = css;
  /// BUILD end
}

export async function updateTheme(theme: string) {
  const css = await invoke(IPC.UPDATE_THEME, theme);
  if (!css) return;
  updateThemeCss(css);
}

export async function updateCurrentTheme() {
  updateThemeCss(await invoke(IPC.GET_THEME));
}

export function isFocusElement(el?: EventTarget | null) {
  const tags = ["INPUT", "SELECT", "BUTTON", "SUMMARY"];
  return tags.includes((el as HTMLElement)?.tagName ?? "");
}

export const setLikedView = (liked: boolean, element: HTMLElement) => {
  element.textContent = liked ? "♥" : "♡";
  element.className = `fav ${liked ? "liked" : ""}`;
};

export function capitalize(str: string) {
  const first = str.charAt(0).toUpperCase();
  return first + str.slice(1).toLowerCase();
}

export function startDrag(path: string) {
  invoke(IPC.START_DRAG_FILE, path);
}

export function debounce<T extends (...args: any[]) => void>(
  fn: T,
  delay: number,
): (...args: Parameters<T>) => void {
  let timeoutId: ReturnType<typeof setTimeout> | undefined;

  return function (...args: Parameters<T>): void {
    if (timeoutId !== undefined) {
      clearTimeout(timeoutId);
    }
    timeoutId = setTimeout(() => {
      fn(...args);
    }, delay);
  };
}
