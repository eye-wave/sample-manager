import { $el, d } from "../alias";
import { getPluginPaths, getSampleFolders } from "../api";
import { search } from "../browse/browse";
import { PaginationHandler } from "../browse/pagination";
import { SEPARATOR } from "../helpers";
import { IPC, invoke, listen } from "../invoke/invoke";
import { playerHandle } from "../player/player";
import { initSidebarResize } from "./resize";
import { FileTree } from "./tree";

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

// --- hover popup ---
const popup = $el("div");
popup.className = "tree-section item popup";
d.body.appendChild(popup as Node);

const hidePopup = () => {
  popup.style.display = "none";
};
const onHover = (e: Event) => {
  const el = e.target as HTMLDivElement;
  if (!el?.hasAttribute("data-path")) return hidePopup();
  popup.textContent = el.textContent;
  popup.style.display = "";
  popup.style.top = el.offsetTop + 65.5 - sidebar__.scrollTop + "px";
  popup.style.left = el.offsetLeft - 8 + "px";
};

hidePopup();
sidebar__.onmouseenter = onHover;
sidebar__.onmousemove = onHover;
sidebar__.onscroll = hidePopup;
sidebar__.onmouseleave = hidePopup;
// @ts-expect-error
sidebar__.parentElement.onwheel = hidePopup;

// --- initial load ---
invoke(IPC.StartSampleScan);

Promise.all([getPluginPaths(), getSampleFolders()]).then(async ([plugins, sampleFolders]) => {
  for (const plugin of plugins) await tree.addPluginFolder(plugin);
  for (const folder of sampleFolders) await tree.addRealFolder(folder);
});

// --- click to play ---
sidebar__.onclick = (e: Event) => {
  const url = (e.target as HTMLElement)?.dataset.path;
  if (!url) return;
  playerHandle.play(decodeURI(url));
};

// --- add folder ---
add_folder__.onclick = async () => {
  const folder = await invoke(IPC.OpenFolder);
  if ((await invoke(IPC.AddSampleFolder, folder)) !== "Ok") return;
  invoke(IPC.StartSampleScan, folder);
  await tree.addRealFolder(folder);
};

// --- plugin lifecycle ---
listen("plug-rm", async () => {
  const remaining = await getPluginPaths();
  const remainingPaths = new Set(remaining.map((p) => p.path));
  for (const [path] of tree.pluginEntries) {
    if (!remainingPaths.has(path)) tree.removePluginFolder(path);
  }
});

listen("plug-add", async () => {
  const current = await getPluginPaths();
  for (const plugin of current) await tree.addPluginFolder(plugin);
});

listen("plug-download", (changedPath: string) => {
  for (const [pluginPath] of tree.pluginEntries) {
    if (changedPath.startsWith(pluginPath + SEPARATOR) || changedPath === pluginPath) {
      tree.refreshPluginFolder(pluginPath, changedPath);
      return;
    }
  }
});

// --- tab switching ---
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
