import { basename, escapeHTML } from "../helpers";
import { POOL_SIZE } from "./browse";
import type { BrowseRow } from "./row";

export const TagInput = (
  input: HTMLInputElement,
  container: HTMLElement,
  pool: BrowseRow[],
) => {
  const tags: string[] = [];

  const addTag = (tag: string) => {
    tags.push(tag);

    const item = document.createElement("span");
    item.className = "tag-chip";

    const label = escapeHTML(tag);
    item.innerHTML = label + '<span class="chip-x">x</span>';

    item.onclick = () => removeTag(+(item.dataset.i ?? 0));

    container.firstChild
      ? container.insertBefore(item, container.firstChild)
      : container.appendChild(item);
  };

  const removeTag = (i: number) => {
    tags.splice(i, 1);
    container.children.item(i)?.remove();

    for (let j = tags.length - 1; j >= 0; j--) {
      const el = container.children.item(j) as HTMLElement;
      if (el) el.dataset.i = "" + j;
    }
  };

  input.onkeydown = (e) => {
    if (e.key === "Enter") {
      for (const w of input.value.split(/\s+/)) {
        if (!tags.includes(w)) addTag(w);
      }
      input.value = "";
    }
  };

  input.oninput = async () => {
    const q = input.value;

    if (!q.length) {
      pool.forEach((p) => {
        p.hide();
      });

      return;
    }

    const text = await invoke("search_for_sample", tags.reduce((q, t) => q + t + ",", "") + q);

    const lines = text.split("\n").filter(Boolean);

    for (let i = 0; i < POOL_SIZE; i++) {
      const row = pool[i];

      if (i < lines.length) {
        const path = lines[i];
        row.update(basename(path), null, false, []);
        row.setPath(path);
      } else {
        row.hide();
      }
    }
  };
};
