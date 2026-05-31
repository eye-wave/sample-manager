import { invoke, IPC } from "../invoke/invoke";
import { parseVFS } from "./parse";
import { renderNode } from "./render";
import { NodeType } from "./sidebar";
import type { VFSChild, VFSNode } from "./vfs";

export async function loadNode(node: VFSNode): Promise<void> {
  if (node.loaded) return;
  node.loaded = true;

  const fresh: VFSChild[] = await invoke(IPC.ReadDir, node.path()).then((res) =>
    res
      .split("\n")
      .filter((e) => e)
      .map((p) => parseVFS(node.path(), p)),
  );

  const existingPaths = new Set(
    node.children.map((c) => (c.nodeType === NodeType.Dir ? c.absolutePath : c.path)),
  );
  const newChildren = fresh.filter((c) => {
    const p = c.nodeType === NodeType.Dir ? c.absolutePath : c.path;
    return !existingPaths.has(p);
  });

  node.extend(newChildren);
  node.updateCount();
  node.propagateCount();

  if (node.visual?.childrenEl) {
    for (const child of node.children) {
      renderNode(node.visual.childrenEl, child);
    }
  }
}

export async function prefetchCount(node: VFSNode): Promise<void> {
  if (node.loaded) return;

  try {
    const count = +(await invoke(IPC.GetFileCountInDir, node.path()));
    node.visual?.updateCount(count);
  } catch {}
}
