import { $el, d } from "../alias";
import { getPluginPaths, getSampleFolders } from "../api";
import { IPC, invoke } from "../invoke/invoke";
import { onPluginAdded, onPluginDownload, onPluginRemoved, onScanTick } from "@state";
import { SEPARATOR } from "../helpers";
import { FileTree } from "./tree";
import { initSidebarResize } from "./resize";
import { PaginationHandler } from "../browse/pagination";

export enum NodeType {
  File,
  Dir,
}
export enum NodeKind {
  Root,
  Real,
  Plugin,
}

export const TabLibrary = 0 as const;
export const TabFavorites = 1 as const;
export const TabHandle: { tab: 0 | 1 } = { tab: TabLibrary };

export const createSidebar = (
  play: (path: string) => void,
  search: (query: string, tags: string[], fav?: boolean) => void,
) => {
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

  onScanTick.on((n) => {
    s_total__.textContent = n;
  });

  invoke(IPC.StartSampleScan);

  Promise.all([getPluginPaths(), getSampleFolders()]).then(async ([plugins, folders]) => {
    for (const plugin of plugins) await tree.addPluginFolder(plugin);
    for (const folder of folders) await tree.addRealFolder(folder);
  });

  sidebar__.onclick = (e: Event) => {
    const url = (e.target as HTMLElement)?.dataset.path;
    if (!url) return;
    play(decodeURI(url));
  };

  add_folder__.onclick = async () => {
    const folder = await invoke(IPC.OpenFolder);
    if ((await invoke(IPC.AddSampleFolder, folder)) !== "Ok") return;
    invoke(IPC.StartSampleScan, folder);
    await tree.addRealFolder(folder);
  };

  onPluginRemoved.on(async () => {
    const remaining = await getPluginPaths();
    const remainingPaths = new Set(remaining.map((p) => p.path));
    for (const [path] of tree.pluginEntries) {
      if (!remainingPaths.has(path)) tree.removePluginFolder(path);
    }
  });

  onPluginAdded.on(async () => {
    const current = await getPluginPaths();
    for (const plugin of current) await tree.addPluginFolder(plugin);
  });

  onPluginDownload.on((changedPath) => {
    for (const [pluginPath] of tree.pluginEntries) {
      if (changedPath.startsWith(pluginPath + SEPARATOR) || changedPath === pluginPath) {
        tree.refreshPluginFolder(pluginPath, changedPath);
        return;
      }
    }
  });

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
};
