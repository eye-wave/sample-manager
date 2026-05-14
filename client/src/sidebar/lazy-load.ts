import * as IPC from "../gen/ipc-gen";
import { invoke } from "../invoke/invoke";
import { parseVFS } from "./parse";
import { renderNode } from "./render";
import type { VFSChild, VFSNode } from "./vfs";

export async function loadNode(node: VFSNode): Promise<void> {
  if (node.loaded) return;
  node.loaded = true;

  const children: VFSChild[] = await invoke(IPC.READ_DIR, node.path()).then((res) =>
    res
      .split("\n")
      .filter((e) => e)
      .map((p) => parseVFS(node.path(), p)),
  );

  node.extend(children);
  node.updateCount();
  node.propagateCount();

  if (node.visual?.childrenEl) {
    for (const child of node.children) {
      renderNode(node.visual.childrenEl, child);
    }
  }
}
