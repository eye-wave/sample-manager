import { PreviewHandler } from "../preview";
import { TagInput } from "./input";
import { BrowseRow } from "./row";

declare const list_scroll: HTMLDivElement;
declare const search: HTMLInputElement;
declare const search_tags: HTMLInputElement;

export const POOL_SIZE = 100;

let lastSelected = 0;
function onSelect(i: number, file: string) {
  invoke("read_audio_file", file);
  invoke("play_audio_file", file);

  pool[lastSelected]?.highlight(false);

  const current = pool[i];
  if (!current) return;

  current.highlight(true);

  PreviewHandler.label = current.name;
  PreviewHandler.img = "";
  PreviewHandler.tags = current.tags;

  lastSelected = i;
}

const pool: BrowseRow[] = Array.from({ length: POOL_SIZE }, (_, i) => BrowseRow(i, onSelect));
const fragment = document.createDocumentFragment();

TagInput(search, search_tags, pool);

pool.forEach((item) => {
  fragment.appendChild(item.el);
});

list_scroll.appendChild(fragment);
