import { $el, d } from "../alias";
import { getPluginPaths, getSampleFolders } from "../api";
import { clearHighlight, search } from "../browse/browse";
import { PaginationHandler } from "../browse/pagination";
import { emit, BusEvent } from "../bus";
import { invoke, IPC, listen } from "../invoke/invoke";
import { FileTree } from "./tree";
import { initSidebarResize } from "./resize";

export enum NodeType {
  File,
  Dir,
}
export enum NodeKind {
  Root,
  Real,
  Plugin,
}

initSidebarResize();

const tree = new FileTree(sidebar__);

// Hover popup

const popup = $el("div");
popup.className = "tree-section item popup";
d.body.appendChild(popup as Node);

function hidePopup() {
  popup.style.display = "none";
}

function onHover(e: Event) {
  const el = e.target as HTMLDivElement;
  if (!el?.hasAttribute("data-path")) return hidePopup();

  popup.textContent = el.textContent;
  popup.style.display = "";
  popup.style.top = el.offsetTop + 65.5 - sidebar__.scrollTop + "px";
  popup.style.left = el.offsetLeft - 8 + "px";
}

hidePopup();
sidebar__.onmouseenter = onHover;
sidebar__.onmousemove = onHover;
sidebar__.onscroll = hidePopup;
sidebar__.onmouseleave = hidePopup;

// @ts-expect-error
sidebar__.parentElement.onwheel = hidePopup;

// Initial load

invoke(IPC.StartSampleScan);

Promise.all([getPluginPaths(), getSampleFolders()]).then(async ([plugins, sampleFolders]) => {
  for (const plugin of plugins) {
    await tree.addPluginFolder(plugin);
  }

  for (const folder of sampleFolders) {
    await tree.addRealFolder(folder);
  }
});

// Click to play

sidebar__.onclick = async (e: Event) => {
  const url = (e.target as HTMLElement)?.dataset.path;
  if (!url) return;

  emit(BusEvent.PlaySong, decodeURI(url));
  clearHighlight();
};

// Add folder button

add_folder__.onclick = async () => {
  const folder = await invoke(IPC.OpenFolder);
  if ((await invoke(IPC.AddSampleFolder, folder)) !== "Ok") return;

  invoke(IPC.StartSampleScan, folder);
  await tree.addRealFolder(folder);
};

// Plugin lifecycle events

listen("plug-rm", async () => {
  const remaining = await getPluginPaths();
  const remainingPaths = new Set(remaining.map((p) => p.path));

  const toRemove: string[] = [];
  for (const [path] of (tree as FileTree).pluginEntries as Map<string, unknown>) {
    if (!remainingPaths.has(path)) toRemove.push(path);
  }

  for (const path of toRemove) {
    tree.removePluginFolder(path);
  }
});

listen("plug-add", async () => {
  const current = await getPluginPaths();
  for (const plugin of current) {
    await tree.addPluginFolder(plugin);
  }
});

listen("plug-download", (changedPath: string) => {
  for (const [pluginPath] of tree.pluginEntries) {
    if (changedPath.startsWith(pluginPath)) {
      tree.refreshPluginFolder(pluginPath, changedPath).then(() => {});
      return;
    }
  }
});

// Tab switching

export const TabLibrary = 0 as const;
export const TabFavorites = 1 as const;

export const TabHandle: { tab: 0 | 1 } = { tab: TabLibrary };

tlib__.onchange = () => {
  if (!tlib__.checked) return;
  TabHandle.tab = TabLibrary;
  PaginationHandler.setPage(1);
  search("", [], false);
};

tfav__.onchange = () => {
  if (!tfav__.checked) return;
  TabHandle.tab = TabFavorites;
  PaginationHandler.setPage(1);
  search("", [], true);
};
