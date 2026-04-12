import { loadNode } from "./lazy-load";
import { SIDEBAR_ITEM } from "./template";
import type { VFSNode } from "./vfs";

export function renderNode(parent: HTMLElement, node: VFSNode): void {
  parent.insertAdjacentHTML("beforeend", SIDEBAR_ITEM(true, node.name));
  parent.insertAdjacentHTML(
    "beforeend",
    '<div class="tree-children" style="display:none"></div>',
  );

  const section = parent.lastElementChild!.previousElementSibling!;
  node.bind(section);
  node.updateCount();

  node.labelEl?.addEventListener("click", async () => {
    // Fetch on first open if empty.
    if (!node.loaded && node.children.length === 0) {
      await loadNode(node);
    }
    node.toggle();
  });

  // Render nodes known at construction time
  if (node.children.length > 0) {
    node.loaded = true;
    for (const child of node.children) {
      if (typeof child === "string") {
        node.childrenEl!.insertAdjacentHTML(
          "beforeend",
          SIDEBAR_ITEM(false, child.split(/[\\/]/).at(-1) ?? child),
        );
      } else {
        renderNode(node.childrenEl!, child);
      }
    }
  }
}
