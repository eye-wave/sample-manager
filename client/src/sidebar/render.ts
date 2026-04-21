import { ADD_EVENT_LISTENER, BEFOREEND, INSERT_ADJACENT_HTML } from "../alias";
import { basename } from "../helpers";
import { loadNode } from "./lazy-load";
import { SIDEBAR_FOLDER, SIDEBAR_ITEM } from "./template";
import { FileType, type VFSNode } from "./vfs";

export function renderNode(parent: HTMLElement, node: VFSNode): void {
  parent[INSERT_ADJACENT_HTML](BEFOREEND, SIDEBAR_FOLDER(node.displayName));
  parent[INSERT_ADJACENT_HTML](
    BEFOREEND,
    /* HTML */ `<div class="tree-children" style="display:none"></div>`,
  );

  const section = parent.lastElementChild?.previousElementSibling;
  if (!section) return;

  node.bind(section);
  node.updateCount();

  node.visual?.labelEl?.[ADD_EVENT_LISTENER]("click", () => {
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
        if (child.nodeType === FileType) {
          node.visual.childrenEl[INSERT_ADJACENT_HTML](
            BEFOREEND,
            SIDEBAR_ITEM(basename(child.path), child.ftype, child.path),
          );
        } else {
          renderNode(node.visual.childrenEl, child);
        }
      }
    }
  }
}
