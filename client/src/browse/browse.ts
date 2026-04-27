import { APPEND_CHILD, d } from "../alias";
import { playerHandle } from "../player/player";
import { TagInput } from "./input";
import { BrowseRow } from "./row";

declare const list_scroll: HTMLDivElement;
declare const search: HTMLInputElement;
declare const search_tags: HTMLInputElement;

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
const fragment = d.createDocumentFragment();

TagInput(search, search_tags, pool);

pool.forEach((item) => {
  fragment[APPEND_CHILD](item.el);
});

list_scroll[APPEND_CHILD](fragment);
