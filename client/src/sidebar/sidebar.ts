import { $el, d } from "../alias";
import { search } from "../browse/browse";
import * as IPC from "../gen/ipc-gen";
import { basename } from "../helpers";
import { invoke } from "../invoke/invoke";
import { playerHandle } from "../player/player";
import { parseVFS } from "./parse";
import { renderNode } from "./render";
import { NodeType, type VFSChild, VFSNode } from "./vfs";

declare const sidebar__: HTMLDivElement;
declare const add_folder__: HTMLButtonElement;

const popup = $el("div");
popup.className = "tree-section item popup";

d.body.appendChild(popup);

const root = VFSNode.root("__root__");

function hidePopup() {
  popup.style.display = "none";
}

function onHover(e: Event) {
  const el = e.target as HTMLDivElement;
  if (!el?.hasAttribute("data-path")) return hidePopup();

  popup.textContent = el.textContent;
  popup.style.display = "";
  popup.style.top = el.offsetTop - sidebar__.scrollTop + "px";
  popup.style.left = el.offsetLeft - 8 + "px";
}

hidePopup();
sidebar__.onmouseenter = onHover;
sidebar__.onmousemove = onHover;
sidebar__.onscroll = hidePopup;
sidebar__.onmouseleave = hidePopup;

invoke(IPC.START_SAMPLE_SCAN);
invoke(IPC.GET_SAMPLE_FOLDERS).then(async (res) => {
  const folders: string[] = res.split("\n").filter((e) => e);

  for (const folder of folders) {
    const node = VFSNode.root(folder);

    const children: VFSChild[] = await invoke(IPC.READ_DIR, folder).then((res) =>
      res
        .split("\n")
        .filter((e) => e)
        .map((p) => parseVFS(folder, p)),
    );

    node.extend(children);
    root.add(node);
  }

  for (const child of root.children) {
    if (child.nodeType === NodeType) {
      renderNode(sidebar__, child);
    }
  }
});

sidebar__.onclick = async (e) => {
  const url = (e.target as HTMLElement)?.dataset.path;
  if (!url) return;

  const path = decodeURI(url);

  playerHandle.startPlaying(path, basename(path));
};

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
