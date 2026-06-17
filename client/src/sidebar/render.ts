import { basename, startDrag } from "../helpers";
import { IPC, invoke } from "../invoke/invoke";
import { NodeType } from "./sidebar";
import { SIDEBAR_FOLDER, SIDEBAR_ITEM } from "./template";
import { loadNode } from "./tree";
import type { VFSChild, VFSNode } from "./vfs";

export async function prefetchCount(node: VFSNode): Promise<void> {
  if (node.loaded) return;
  try {
    const count = +(await invoke(IPC.GetFileCountInDir, node.path()));
    node.visual?.updateCount(count);
  } catch {}
}

export function renderNode(parent: HTMLElement, node: VFSChild, icon?: string): void {
  if (node.nodeType === NodeType.File) {
    parent.insertAdjacentHTML(
      "beforeend",
      SIDEBAR_ITEM(basename(node.path), node.ftype, node.path),
    );
    const el = parent.querySelector<HTMLDivElement>(`[data-path="${encodeURI(node.path)}"]`);
    if (el) {
      el.draggable = true;
      el.ondragstart = () => startDrag(node.path);
    }
    return;
  }

  parent.insertAdjacentHTML("beforeend", SIDEBAR_FOLDER(node.displayName, icon));
  parent.insertAdjacentHTML(
    "beforeend",
    `<div class="tree-children" style="display:none"></div>`,
  );

  const section = parent.lastElementChild?.previousElementSibling;
  if (!section) return;

  node.bind(section);
  node.updateCount();

  node.visual?.labelEl?.addEventListener("click", () => {
    if (!node.loaded) {
      loadNode(node).then(() => node.toggle());
    } else {
      if (node.visual?.childrenEl && node.visual.childrenEl.children.length === 0) {
        for (const child of node.children) renderNode(node.visual.childrenEl, child);
      }
      node.toggle();
    }
  });

  if (node.children.length > 0) {
    node.loaded = true;
    for (const child of node.children) {
      if (node.visual?.childrenEl) renderNode(node.visual.childrenEl, child);
    }
  }
}
