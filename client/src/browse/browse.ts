import { basename } from "../helpers";
import { BrowseRow } from "./row";

declare const list_scroll: HTMLDivElement;
declare const search: HTMLInputElement;

const POOL_SIZE = 100;

function onSelect(file: string) {
  invoke("read_audio_file", file);
}

const pool: BrowseRow[] = Array.from({ length: POOL_SIZE }, () => new BrowseRow(onSelect));
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
      const path = lines[i]!;

      pool[i]?.update(basename(path), null, false, []);
      pool[i]?.setPath(path);
    } else {
      pool[i]?.hide();
    }
  }
};
