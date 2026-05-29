import type { SampleEntry } from "@typegen/SampleEntry";
import { BUS } from "../bus";
import { basename, setLikedView } from "../helpers";
import { invoke, IPC, listen } from "../invoke/invoke";
import { TagInput } from "./input";
import { PaginationHandler } from "./pagination";
import { BrowseRow } from "./row";
import { PreviewHandler } from "../preview/preview";
import { emit } from "../bus";
import { callSampleSearch, toggleFav } from "../api";

declare const list_scroll__: HTMLDivElement;
declare const search__: HTMLInputElement;
declare const search_tags__: HTMLInputElement;
declare const pagination__: HTMLDivElement;
declare const online_plugin_btn__: HTMLButtonElement;

export const POOL_SIZE = 100;

let isOnlineSearch = false;

invoke(IPC.ANY_ONLINE_PLUGIN_LOADED).then((loaded) => {
  if (!+loaded) return;

  online_plugin_btn__.style.display = "";
});

online_plugin_btn__.onclick = () => {
  isOnlineSearch = !isOnlineSearch;
  if (isOnlineSearch) {
    online_plugin_btn__.classList.remove("btn-surface");
    online_plugin_btn__.classList.add("btn-primary");
  } else {
    online_plugin_btn__.classList.remove("btn-primary");
    online_plugin_btn__.classList.add("btn-surface");
  }
};

let lastSelected = 0;
function onSelect(i: number, file: string) {
  pool[lastSelected]?.highlight(false);

  const current = pool[i];
  if (!current) return;

  current.highlight(true);
  emit(BUS.PLAY_SONG, file);

  lastSelected = i;
}

export function clearHighlight() {
  pool[lastSelected]?.highlight(false);
  lastSelected = 0;
}

export function syncHighlight() {
  const playingPath = PreviewHandler.path;

  for (let i = 0; i < POOL_SIZE; i++) {
    const row = pool[i];
    const isPlaying = !!playingPath && row.path === playingPath;
    row.highlight(isPlaying);
    if (isPlaying) lastSelected = i;
  }
}

const pool: BrowseRow[] = Array.from({ length: POOL_SIZE }, (_, i) =>
  BrowseRow(i, onSelect, (p) => toggleFav(p)),
);

TagInput(search__, search_tags__, pool);

for (const item of pool) {
  list_scroll__.insertBefore(item.el as Node, pagination__);
}

listen("set-fav", (payload) => {
  const fav = !!+payload.charAt(0);
  const path = payload.slice(1);

  for (const el of pool) {
    if (el.path !== path) continue;

    setLikedView(fav, el.favEl);
  }
});

const PAGE_SIZE = 50;
export async function search(query: string, tags: string[], fav = false) {
  callSampleSearch({
    query,
    limit: PAGE_SIZE,
    offset: (PaginationHandler.page - 1) * PAGE_SIZE,
    tags,
    isFav: fav,
  });

  PaginationHandler.display(true);
}

listen("search", (payload) => {
  const { files, count }: { files: SampleEntry[]; count: number } = (() => {
    try {
      return JSON.parse(payload);
    } catch (_) {
      return { count: 0, files: [] };
    }
  })();

  PaginationHandler.setPages((count / PAGE_SIZE) | 0);

  for (let i = 0; i < POOL_SIZE; i++) {
    const row = pool[i];

    if (i < files.length) {
      const item = files[i];

      row.update(basename(item.name), null, item.is_fav, item.tags);
      if (item.path) row.setPath(item.path);
    } else {
      row.hide();
    }
  }

  syncHighlight();
});
