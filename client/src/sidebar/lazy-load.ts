import { renderNode } from "./render";
import { SIDEBAR_ITEM } from "./template";
import { type VFSChild, VFSNode } from "./vfs";

export async function loadNode(node: VFSNode): Promise<void> {
  if (node.loaded) return;
  node.loaded = true;

  console.log("trying to read", node.path());

  const children: VFSChild[] = await invoke("read_dir", node.path()).then((res) =>
    res
      .split("\n")
      .filter(Boolean)
      .map((line): VFSChild => {
        const isDir = line.charAt(0) === "1";
        const path = line.slice(1);
        return isDir ? VFSNode.child(path) : path;
      }),
  );

  node.extend(children);
  node.updateCount();
  node.propagateCount();

  if (node.childrenEl) {
    for (const child of node.children) {
      if (typeof child === "string") {
        node.childrenEl.insertAdjacentHTML(
          "beforeend",
          SIDEBAR_ITEM(false, child.split(/[\\/]/).at(-1) ?? child),
        );
      } else {
        renderNode(node.childrenEl, child);
      }
    }
  }
}
