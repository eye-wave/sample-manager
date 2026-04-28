import { $el, w } from "../alias";
import { basename } from "../helpers";
import { POOL_SIZE } from "./browse";
import { PaginationHandler } from "./pagination";
import type { BrowseRow } from "./row";

declare const list_scroll__: HTMLDivElement;

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

  let recentQuery = "";

  w.addEventListener("keydown", (e) => {
    if (e.key === "/" || ((e.key === "k" || e.key === "K") && e.ctrlKey)) {
      e.preventDefault();
      input.focus();
    }
  });

  const addTag = (tag: string) => {
    if (!tag) return;
    tags.push(tag);

    const item = $el("span");

    item.className = "tag x";
    item.textContent = tag + " x";

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
    if (e.key === "Escape") {
      input.blur();
      return;
    }
    if (e.key === "Enter") {
      for (const w of input.value.split(/\s+/)) {
        if (!tags.includes(w)) addTag(w);
      }
      input.value = "";
    }
  };

  input.oninput = () => {
    const query = input.value;

    if (!query.length) {
      recentQuery = "";

      for (const p of pool) p.hide();
      PaginationHandler.display(false);

      return;
    }

    recentQuery = query;

    list_scroll__.scrollTo({ top: 0 });
    search(query, tags, 1);
  };

  async function search(query: string, tags: string[], offset: number) {
    const PAGE_SIZE = 50;

    const text = await invoke(
      "search_for_sample",
      [PAGE_SIZE, (offset - 1) * PAGE_SIZE, ...tags, query].join(","),
    );
    const { files, count }: { files: FSSample[]; count: number } = (() => {
      try {
        return JSON.parse(text);
      } catch (_) {
        return [];
      }
    })();

    PaginationHandler.display(true);
    PaginationHandler.setPages((count / PAGE_SIZE) | 0);
    PaginationHandler.setPage(offset);

    for (let i = 0; i < POOL_SIZE; i++) {
      const row = pool[i];

      if (i < files.length) {
        const item = files[i];

        row.update(basename(item.path), null, false, item.tags);
        row.setPath(item.path);
      } else {
        row.hide();
      }
    }
  }

  PaginationHandler.onClick = (p) => search(recentQuery, tags, p);
};
