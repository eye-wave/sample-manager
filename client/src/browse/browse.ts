import type { SampleEntry } from "@typegen/SampleEntry";
import { callOnlineSearch, callSampleSearch, downloadFile, toggleFav } from "../api";
import { basename, debounce, setLikedView } from "../helpers";
import { IPC, invoke } from "../invoke/invoke";
import { BrowseRow, type OnSelectProps } from "./row";
import { TagInput } from "./input";
import { PaginationHandler } from "./pagination";
import { onFavChanged } from "@state";
import type { Readable, Writable } from "../store";

export const POOL_SIZE = 100;

type BrowseStores = {
  currentPath: Writable<string | null>;
  searchResults: Readable<SampleEntry[]>;
  resultCount: Readable<number>;
};

export const createBrowse = (stores: BrowseStores, play: (path: string) => void) => {
  let isOnlineSearch = false;

  // --- online plugin button ---
  const showOnlineBtn = () => {
    invoke(IPC.AnyOnlinePluginLoaded).then((loaded) => {
      if (!+loaded) return;
      online_plugin_btn__.style.display = "";
    });
  };

  showOnlineBtn();

  online_plugin_btn__.onclick = () => {
    isOnlineSearch = !isOnlineSearch;
    online_plugin_btn__.classList.toggle("btn-surface", !isOnlineSearch);
    online_plugin_btn__.classList.toggle("btn-primary", isOnlineSearch);
  };

  // --- pool ---
  let lastSelected = 0;

  const syncHighlight = () => {
    const playingPath = stores.currentPath.get();
    for (let i = 0; i < POOL_SIZE; i++) {
      const row = pool[i];
      const isPlaying = !!playingPath && row.path === playingPath;
      row.highlight(isPlaying);
      if (isPlaying) lastSelected = i;
    }
  };

  const onSelect = async (i: number, props: OnSelectProps) => {
    const path = props.type === "plug" ? await downloadFile(props) : props.path;
    pool[lastSelected]?.highlight(false);
    const current = pool[i];
    if (!current) return;
    current.highlight(true);
    lastSelected = i;
    play(path);
  };

  const pool: BrowseRow[] = Array.from({ length: POOL_SIZE }, (_, i) =>
    BrowseRow(i, onSelect, (p) => toggleFav(p)),
  );

  TagInput(search__, search_tags__, pool);

  for (const item of pool) {
    list_scroll__.insertBefore(item.el as Node, pagination__);
  }

  stores.searchResults.subscribe((files) => {
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

  stores.resultCount.subscribe((count) => {
    PaginationHandler.setPages((count / PAGE_SIZE) | 0);
  });

  // sync highlight whenever currentPath changes (e.g. sidebar click)
  stores.currentPath.subscribe(syncHighlight);

  // --- fav ---
  onFavChanged.on(({ path, fav }) => {
    for (const row of pool) {
      if (row.path === path) setLikedView(fav, row.favEl);
    }
  });

  // --- search ---
  const PAGE_SIZE = 50;
  const debouncedOnlineSearch = debounce(callOnlineSearch, 350);

  const search = (query: string, tags: string[], fav = false) => {
    const params = {
      query,
      limit: PAGE_SIZE,
      offset: (PaginationHandler.page - 1) * PAGE_SIZE,
      tags,
      isFav: fav,
    };
    (isOnlineSearch ? debouncedOnlineSearch : callSampleSearch)(params);
    PaginationHandler.display(true);
  };

  return { search, syncHighlight };
};
