import { TagInput } from "./input";
import { BrowseRow } from "./row";

declare const list_scroll: HTMLDivElement;
declare const search: HTMLInputElement;
declare const search_tags: HTMLInputElement;

export const POOL_SIZE = 100;

function onSelect(file: string) {
  invoke("read_audio_file", file);
}

const pool: BrowseRow[] = Array.from({ length: POOL_SIZE }, () => new BrowseRow(onSelect));
const fragment = document.createDocumentFragment();

new TagInput(search, search_tags, pool);

pool.forEach((item) => {
  fragment.appendChild(item.el);
});

list_scroll.appendChild(fragment);
