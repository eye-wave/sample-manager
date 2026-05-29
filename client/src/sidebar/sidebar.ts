import { $el, d } from "../alias";
import { getPluginPaths, getSampleFolders } from "../api";
import { clearHighlight, search } from "../browse/browse";
import { PaginationHandler } from "../browse/pagination";
import { emit } from "../bus";
import { BUS } from "../bus";
import { invoke, IPC, listen } from "../invoke/invoke";
import { parseVFS } from "./parse";
import { renderNode } from "./render";
import { initSidebarResize } from "./resize";
import { NodeType, type VFSChild, VFSNode } from "./vfs";

initSidebarResize();

const popup = $el("div");
popup.className = "tree-section item popup";

d.body.appendChild(popup as Node);

const root = VFSNode.root("__root__");

// plugin nodes are always inserted before this element,
// keeping them above sample folder nodes which follow it.
const pluginAnchor = $el("div");
sidebar__.appendChild(pluginAnchor as Node);

const pluginNodeEls = new Map<string, { section: Element; children: Element | null }>();

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

// Insert a plugin folder node directly before the sentinel so it stays
// above any sample folder nodes that were appended after the anchor.
async function addPluginNode(folder: { name: string; path: string; icon: string | null }) {
  const { path } = folder;

  if (pluginNodeEls.has(path)) return; // already rendered

  const node = VFSNode.root(path, folder.name);

  const children: VFSChild[] = await invoke(IPC.READ_DIR, path).then((res) =>
    res
      .split("\n")
      .filter((e) => e)
      .map((p) => parseVFS(path, p)),
  );

  node.extend(children);
  root.add(node);

  if (node.nodeType !== NodeType) return;

  // Create a temporary container, render into it, then move the resulting
  // nodes before the anchor so they sit at the top of the plugin section.
  const tmp = $el("div");
  renderNode(tmp, node, folder.icon ?? undefined);

  const sectionEl = tmp.children[0] ?? null;
  const childrenEl = tmp.children[1] ?? null;

  if (sectionEl) sidebar__.insertBefore(sectionEl, pluginAnchor);
  if (childrenEl) sidebar__.insertBefore(childrenEl, pluginAnchor);

  if (sectionEl) pluginNodeEls.set(path, { section: sectionEl, children: childrenEl });
}

async function renderRootDirs(
  target: HTMLElement,
  folders: ({ name: string; path: string; icon: string | null } | string)[],
) {
  for (const folder of folders) {
    const path = typeof folder === "string" ? folder : folder.path;

    // @ts-expect-error
    const node = VFSNode.root(path, folder?.name);

    const children: VFSChild[] = await invoke(IPC.READ_DIR, path).then((res) =>
      res
        .split("\n")
        .filter((e) => e)
        .map((p) => parseVFS(path, p)),
    );

    node.extend(children);
    root.add(node);

    if (node.nodeType === NodeType) {
      // @ts-expect-error
      renderNode(target, node, folder?.icon);
    }
  }
}

let loadedPluginPaths: string[] = [];

invoke(IPC.START_SAMPLE_SCAN);
Promise.all([getPluginPaths(), getSampleFolders()]).then(async ([plugins, sampleFolders]) => {
  loadedPluginPaths = plugins.map((p) => p.path);

  for (const plugin of plugins) await addPluginNode(plugin);

  await renderRootDirs(sidebar__, sampleFolders);
});

async function onClick(e: Event) {
  const url = (e.target as HTMLElement)?.dataset.path;
  if (!url) return;

  const path = decodeURI(url);

  emit(BUS.PLAY_SONG, path);
  clearHighlight();
}

sidebar__.onclick = onClick;

add_folder__.onclick = async () => {
  const folder = await invoke(IPC.OPEN_FOLDER);
  if ((await invoke(IPC.ADD_SAMPLE_FOLDER, folder)) !== "Ok") {
    return;
  }

  invoke(IPC.START_SAMPLE_SCAN, folder);

  const node = VFSNode.root(folder);
  root.add(node);
  renderNode(sidebar__, node);
};

export const TabLibrary = 0 as const;
export const TabFavorites = 1 as const;

export const TabHandle: { tab: 0 | 1 } = {
  tab: TabLibrary,
};

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

listen("plug-rm", async () => {
  const remaining = await getPluginPaths();
  const remainingPaths = new Set(remaining.map((p) => p.path));

  for (const path of loadedPluginPaths) {
    if (remainingPaths.has(path)) continue;

    const els = pluginNodeEls.get(path);
    if (els) {
      els.section.remove();
      els.children?.remove();
      pluginNodeEls.delete(path);
    }
  }

  loadedPluginPaths = [...remainingPaths];
});

listen("plug-add", async () => {
  const current = await getPluginPaths();

  const loadedSet = new Set(loadedPluginPaths);
  const newPlugins = current.filter((p) => !loadedSet.has(p.path));

  for (const plugin of newPlugins) {
    await addPluginNode(plugin);
  }

  loadedPluginPaths = current.map((p) => p.path);
});
