import { playerHandle } from "../player/player";
import { TagInput } from "./input";
import { BrowseRow } from "./row";

import "./pagination";

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
  playerHandle.startPlaying(file, current.name, current.tags);

  lastSelected = i;
}

function onDrag(path: string) {
  invoke("start_drag_file", path);
}

const pool: BrowseRow[] = Array.from({ length: POOL_SIZE }, (_, i) =>
  BrowseRow(i, onSelect, onDrag, () => {}),
);

TagInput(search__, search_tags__, pool);

for (const item of pool) {
  list_scroll__.insertBefore(item.el, pagination__);
}
