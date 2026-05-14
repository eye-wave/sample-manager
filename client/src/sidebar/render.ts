import { basename, startDrag } from "../helpers";
import { loadNode } from "./lazy-load";
import { SIDEBAR_FOLDER, SIDEBAR_ITEM } from "./template";
import { FileType, type VFSChild } from "./vfs";

export function renderNode(parent: HTMLElement, node: VFSChild, icon?: string): void {
  if (node.nodeType === FileType) {
    parent.insertAdjacentHTML(
      "beforeend",
      SIDEBAR_ITEM(basename(node.path), node.ftype, node.path),
    );

    parent.querySelectorAll("[data-path]").forEach((i) => {
      const item = i as HTMLDivElement;
      const path = decodeURI(item.dataset.path ?? "");

      if (path) {
        item.draggable = true;
        item.ondragstart = () => startDrag(path);
      }
    });

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
    // Fetch on first open if empty.
    if (!node.loaded && node.children.length === 0) {
      loadNode(node).then(() => node.toggle());
    } else {
      node.toggle();
    }
  });

  // Render nodes known at construction time
  if (node.children.length > 0) {
    node.loaded = true;
    for (const child of node.children) {
      if (node.visual?.childrenEl) {
        renderNode(node.visual.childrenEl, child);
      }
    }
  }
}
