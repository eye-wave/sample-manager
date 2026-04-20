export function basename(name: string) {
  return name.split(/[\\/]/).pop() ?? name;
}

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
