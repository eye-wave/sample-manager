import { basename } from "../helpers";
import { BrowseRow } from "./row";

declare const list_scroll: HTMLDivElement;
declare const search: HTMLInputElement;

const POOL_SIZE = 100;

const pool: BrowseRow[] = Array.from({ length: POOL_SIZE }, () => new BrowseRow());
const fragment = document.createDocumentFragment();

pool.forEach((item) => {
  fragment.appendChild(item.el);
});

list_scroll.appendChild(fragment);

search.oninput = async () => {
  const query = search.value;

  if (query.length === 0) {
    pool.forEach((item) => {
      item.hide();
    });

    return;
  }

  const text = await invoke("search_for_sample", query);
  const lines = text.split("\n").filter(Boolean);

  for (let i = 0; i < POOL_SIZE; i++) {
    if (i < lines.length) {
      // biome-ignore lint/style/noNonNullAssertion: checked before
      pool[i]?.update(basename(lines[i]!), null, false, []);
    } else {
      pool[i]?.hide();
    }
  }
};
