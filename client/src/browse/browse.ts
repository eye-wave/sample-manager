import * as IPC from "../gen/ipc-gen";
import { basename, setLiked, setLikedView } from "../helpers";
import { invoke, listen } from "../invoke/invoke";
import { playerHandle } from "../player/player";
import { TagInput } from "./input";
import { PaginationHandler } from "./pagination";
import { BrowseRow } from "./row";

declare const list_scroll__: HTMLDivElement;
declare const search__: HTMLInputElement;
declare const search_tags__: HTMLInputElement;
declare const pagination__: HTMLDivElement;

export const POOL_SIZE = 100;

let lastSelected = 0;
function onSelect(i: number, file: string) {
  pool[lastSelected]?.highlight(false);

  const current = pool[i];
  if (!current) return;

  current.highlight(true);
  playerHandle.startPlaying(file, current.name, current.isFav, current.tags);

  lastSelected = i;
}

function onDrag(path: string) {
  invoke(IPC.START_DRAG_FILE, path);
}

const pool: BrowseRow[] = Array.from({ length: POOL_SIZE }, (_, i) =>
  BrowseRow(i, onSelect, onDrag, (p, s) => setLiked(p, s)),
);

TagInput(search__, search_tags__, pool);

for (const item of pool) {
  list_scroll__.insertBefore(item.el as Node, pagination__);
}

export function getCurrentSample(): [string, boolean] | null {
  const current = pool[lastSelected];
  if (!current.path) return null;

  return [current.path, current.isFav];
}

listen("set-fav", (payload) => {
  const fav = !!+payload.charAt(0);
  const path = payload.slice(1);

  for (const el of pool) {
    if (el.path !== path) continue;

    setLikedView(fav, el.favEl);
  }
});

export type FSSample = {
  name: string;
  path: string;
  tags: string[];
  fav: boolean;
};

const PAGE_SIZE = 50;
export async function search(query: string, tags: string[], offset: number, fav = false) {
  const params = {
    query,
    limit: PAGE_SIZE,
    offset: (offset - 1) * PAGE_SIZE,
    tags,
    is_fav: fav,
  };

  invoke(IPC.SEARCH_FOR_SAMPLE, params);

  PaginationHandler.display(true);
  PaginationHandler.setPage(offset);
}

listen("search", (payload) => {
  const { files, count }: { files: FSSample[]; count: number } = (() => {
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

      row.update(basename(item.name), null, item.fav, item.tags);
      row.setPath(item.path);
    } else {
      row.hide();
    }
  }
});
