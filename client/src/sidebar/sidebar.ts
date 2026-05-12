import { $el, d } from "../alias";
import { search } from "../browse/browse";
import * as IPC from "../gen/ipc-gen";
import { basename } from "../helpers";
import { invoke } from "../invoke/invoke";
import { playerHandle } from "../player/player";
import { parseVFS } from "./parse";
import { renderNode } from "./render";
import { initSidebarResize } from "./resize";
import { NodeType, type VFSChild, VFSNode } from "./vfs";

initSidebarResize();

declare const sidebar__: HTMLDivElement;
declare const add_folder__: HTMLButtonElement;

const popup = $el("div");
popup.className = "tree-section item popup";

d.body.appendChild(popup as Node);

const root = VFSNode.root("__root__");

function hidePopup() {
  popup.style.display = "none";
}

function onHover(e: Event) {
  const el = e.target as HTMLDivElement;
  if (!el?.hasAttribute("data-path")) return hidePopup();

  popup.textContent = el.textContent;
  popup.style.display = "";
  // Temporary fix i will need to look into why 65.5 offset was needed
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

invoke(IPC.START_SAMPLE_SCAN);
invoke(IPC.GET_SAMPLE_FOLDERS).then(async (res) => {
  const folders: string[] = res.split("\n").filter((e) => e);
  renderRootDirs(sidebar__, folders);
});

invoke(IPC.GET_PLUGIN_PATHS).then(async (res) => {
  renderRootDirs(sidebar__, JSON.parse(res));
});

async function onClick(e: Event) {
  const url = (e.target as HTMLElement)?.dataset.path;
  if (!url) return;

  const path = decodeURI(url);

  playerHandle.startPlaying(path, basename(path));
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

declare const tlib__: HTMLInputElement;
declare const tfav__: HTMLInputElement;

export const TabLibrary = 0 as const;
export const TabFavorites = 1 as const;

export const TabHandle: { tab: 0 | 1 } = {
  tab: TabLibrary,
};

tlib__.onchange = () => {
  const checked = tlib__.checked;
  if (!checked) return;

  TabHandle.tab = TabLibrary;
  search("", [], 1, false);
};

tfav__.onchange = () => {
  const checked = tfav__.checked;
  if (!checked) return;

  TabHandle.tab = TabFavorites;
  search("", [], 1, true);
};
