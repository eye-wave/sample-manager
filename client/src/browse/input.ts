import { $el, APPEND_CHILD, ONCLICK } from "../alias";
import { basename } from "../helpers";
import { POOL_SIZE } from "./browse";
import type { BrowseRow } from "./row";

type FSSample = {
  path: string;
  tags: string[];
};

export const TagInput = (
  input: HTMLInputElement,
  container: HTMLElement,
  pool: BrowseRow[],
) => {
  const tags: string[] = [];

  const addTag = (tag: string) => {
    if (!tag) return;
    tags.push(tag);

    const item = $el("span");

    item.className = "tag x";
    item.textContent = tag + " x";

    item[ONCLICK] = () => removeTag(+(item.dataset.i ?? 0));

    container.firstChild
      ? container.insertBefore(item, container.firstChild)
      : container[APPEND_CHILD](item);
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
    const lines: FSSample[] = (() => {
      try {
        return JSON.parse(text);
      } catch (_) {
        return [];
      }
    })();

    for (let i = 0; i < POOL_SIZE; i++) {
      const row = pool[i];

      if (i < lines.length) {
        const item = lines[i];

        row.update(basename(item.path), null, false, item.tags);
        row.setPath(item.path);
      } else {
        row.hide();
      }
    }
  };
};
