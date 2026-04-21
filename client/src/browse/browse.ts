import { APPEND_CHILD, d } from "../alias";
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

export function displayPreview(path: string, name: string, tags: string[]) {
  invoke("read_audio_file", path);
  invoke("play_audio_file", path);

  PreviewHandler.label = name;
  PreviewHandler.img = "";
  PreviewHandler.tags = tags;
}

const pool: BrowseRow[] = Array.from({ length: POOL_SIZE }, (_, i) => BrowseRow(i, onSelect));
const fragment = d.createDocumentFragment();

TagInput(search, search_tags, pool);

pool.forEach((item) => {
  fragment[APPEND_CHILD](item.el);
});

list_scroll[APPEND_CHILD](fragment);
