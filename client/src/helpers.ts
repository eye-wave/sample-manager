export function escapeHTML(str: string) {
  return str
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#039;");
}

export function renderTags<E extends HTMLElement>(tagsEl: E, tags: string[]) {
  tagsEl.innerHTML = tags.map((t) => /* HTML */ `<span class="tag">${t}</span>`).join("");
}

/// UNIX start
// biome-ignore lint/correctness/noUnusedVariables: trust
var SEPARATOR = "/";
/// UNIX end

/// WIN start
// biome-ignore lint/suspicious/noRedeclare: trust
var SEPARATOR = "\\";
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
