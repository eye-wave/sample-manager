import type { SampleEntry } from "@typegen/SampleEntry";
import { callOnlineSearch, callSampleSearch, downloadFile, toggleFav } from "../api";
import { basename, debounce, setLikedView } from "../helpers";
import { IPC, invoke, listen } from "../invoke/invoke";
import { playerHandle } from "../player/player";
import { PreviewHandler } from "../preview/preview";
import { TagInput } from "./input";
import { PaginationHandler } from "./pagination";
import { BrowseRow, type OnSelectProps } from "./row";

export const POOL_SIZE = 100;

let isOnlineSearch = false;

const showOnlineBtn = () => {
  invoke(IPC.AnyOnlinePluginLoaded).then((loaded) => {
    if (!+loaded) return;
    online_plugin_btn__.style.display = "";
  });
};

showOnlineBtn();
listen("plug-add", showOnlineBtn);

online_plugin_btn__.onclick = () => {
  isOnlineSearch = !isOnlineSearch;
  online_plugin_btn__.classList.toggle("btn-surface", !isOnlineSearch);
  online_plugin_btn__.classList.toggle("btn-primary", isOnlineSearch);
};

let lastSelected = 0;

async function onSelect(i: number, props: OnSelectProps) {
  const path = props.type === "plug" ? await downloadFile(props) : props.path;

  pool[lastSelected]?.highlight(false);
  const current = pool[i];
  if (!current) return;

  current.highlight(true);
  lastSelected = i;

  playerHandle.play(path);
}

// playerHandle fires our onPlay callback whenever any source starts playing,
// so we can sync the highlight state without browse.ts knowing about sidebar.
playerHandle.onPlay(() => syncHighlight());

function syncHighlight() {
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
    if (el.path === path) setLikedView(fav, el.favEl);
  }
});

const PAGE_SIZE = 50;
const debouncedOnlineSearch = debounce(callOnlineSearch, 350);

export function search(query: string, tags: string[], fav = false) {
  const params = {
    query,
    limit: PAGE_SIZE,
    offset: (PaginationHandler.page - 1) * PAGE_SIZE,
    tags,
    isFav: fav,
  };
  (isOnlineSearch ? debouncedOnlineSearch : callSampleSearch)(params);
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
      row.update(basename(item.name), null, item.isFav, item.tags);
      if (item.source === "native") row.setPath(item.path);
      else {
        row.setUrl(item.url, item.id);
        row.name = item.name;
      }
    } else {
      row.hide();
    }
  }

  syncHighlight();
});
