import { $el } from "../alias";

import { addShortcut } from "../shortcuts";
import { TabFavorites, TabHandle } from "../sidebar/sidebar";
import { search } from "./browse";
import { PaginationHandler } from "./pagination";
import type { BrowseRow } from "./row";

declare const list_scroll__: HTMLDivElement;

export const TagInput = (
  input: HTMLInputElement,
  container: HTMLElement,
  pool: BrowseRow[],
) => {
  const tags: string[] = [];

  let recentQuery = "";

  const callback = (e: KeyboardEvent) => {
    e.preventDefault();
    input.focus();
  };

  const description = "Jump to search";
  addShortcut(description, "k", 0b100, callback);
  addShortcut(description, "/", 0, callback);

  const addTag = (tag: string) => {
    if (!tag) return;
    tags.push(tag);

    const item = $el("span");

    item.className = "tag x";
    item.textContent = tag + " x";

    item.onclick = () => removeTag(+(item.dataset.i ?? 0));

    container.firstChild
      ? container.insertBefore(item as Node, container.firstChild)
      : container.appendChild(item as Node);
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
    const isFav = TabHandle.tab === TabFavorites;

    if (!query.length && !isFav) {
      recentQuery = "";
      for (const p of pool) p.hide();
      PaginationHandler.display(false);
      return;
    }

    recentQuery = query;
    list_scroll__.scrollTo({ top: 0 });
    PaginationHandler.setPage(1);
    search(query, tags, isFav);
  };

  PaginationHandler.onClick = () => {
    const isFav = TabHandle.tab === TabFavorites;
    search(recentQuery, tags, isFav);
  };
};
