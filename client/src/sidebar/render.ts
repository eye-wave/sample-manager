import { basename, startDrag } from "../helpers";
import { loadNode } from "./lazy-load";
import { NodeType } from "./sidebar";
import { SIDEBAR_FOLDER, SIDEBAR_ITEM } from "./template";
import type { VFSChild } from "./vfs";

export function renderNode(parent: HTMLElement, node: VFSChild, icon?: string): void {
  if (node.nodeType === NodeType.File) {
    parent.insertAdjacentHTML(
      "beforeend",
      SIDEBAR_ITEM(basename(node.path), node.ftype, node.path),
    );

    // Bind drag only to the newly inserted element, not all [data-path] in parent.
    const encoded = encodeURI(node.path);
    const el = parent.querySelector<HTMLDivElement>(`[data-path="${encoded}"]`);
    if (el) bindDrag(el, node.path);

    return;
  }

  parent.insertAdjacentHTML("beforeend", SIDEBAR_FOLDER(node.displayName, icon));
  parent.insertAdjacentHTML(
    "beforeend",
    /* HTML */ `<div class="tree-children" style="display:none"></div>`,
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
        for (const child of node.children) {
          renderNode(node.visual.childrenEl, child);
        }
      }
      node.toggle();
    }
  });

  if (node.children.length > 0) {
    node.loaded = true;
    for (const child of node.children) {
      if (node.visual?.childrenEl) {
        renderNode(node.visual.childrenEl, child);
      }
    }
  }
}

function bindDrag(el: HTMLDivElement, path: string): void {
  el.draggable = true;
  el.ondragstart = () => startDrag(path);
}
