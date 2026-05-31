import { $el } from "../alias";
import { invoke, IPC } from "../invoke/invoke";
import { parseVFS } from "./parse";
import { prefetchCount } from "./lazy-load";
import { renderNode } from "./render";
import { VFSNode, type VFSChild, type VFSNode as VFSNodeType } from "./vfs";
import { NodeType } from "./sidebar";
import { SEPARATOR } from "../helpers";

export type PluginFolderDef = {
  name: string;
  path: string;
  icon: string | null;
};

export type RealFolderDef = string | { name?: string; path: string; icon?: string | null };

type PluginEntry = {
  node: VFSNodeType;
  sectionEl: Element;
  childrenEl: Element | null;
};

export class FileTree {
  private readonly root: VFSNodeType;
  private readonly anchor: HTMLDivElement;
  private readonly container: HTMLElement;

  readonly pluginEntries = new Map<string, PluginEntry>();

  constructor(container: HTMLElement) {
    this.container = container;
    this.root = VFSNode.root("__root__");

    this.anchor = $el("div") as HTMLDivElement;
    this.container.appendChild(this.anchor);
  }

  // Plugin folders

  /**
   * renders a plugin folder above the anchor sentinel and tracks it.
   * no-ops if the path is already present.
   */
  async addPluginFolder(folder: PluginFolderDef): Promise<void> {
    const { path } = folder;
    if (this.pluginEntries.has(path)) return;

    const node = VFSNode.plugin(path, folder.name);
    const children = await this.readDir(path);

    node.extend(children);
    this.root.add(node);

    if (node.nodeType !== NodeType.Dir) return;

    const tmp = $el("div") as HTMLElement;
    renderNode(tmp, node, folder.icon ?? undefined);

    const sectionEl = tmp.children[0] ?? null;
    const childrenEl = tmp.children[1] ?? null;

    if (sectionEl) this.container.insertBefore(sectionEl, this.anchor);
    if (childrenEl) this.container.insertBefore(childrenEl, this.anchor);

    await prefetchCount(node);

    this.pluginEntries.set(path, { node, sectionEl, childrenEl });
  }

  /**
   * removes a plugin folder's DOM and internal tracking by path.
   * no-ops if the path was never added.
   */
  removePluginFolder(path: string): void {
    const entry = this.pluginEntries.get(path);
    if (!entry) return;

    entry.sectionEl.remove();
    entry.childrenEl?.remove();
    this.pluginEntries.delete(path);
  }

  /**
   * merges new children into the changed subfolder.
   */
  async refreshPluginFolder(pluginPath: string, changedSubpath?: string): Promise<void> {
    const entry = this.pluginEntries.get(pluginPath);
    if (!entry) return;

    const { node } = entry;
    const targetNode = changedSubpath ? await findOrLoadNode(node, changedSubpath) : node;

    if (!targetNode) return;
    if (!targetNode) return;

    const freshChildren = await readDir(targetNode.absolutePath);

    // Use paths already in the VFS as the existing set — but re-check against
    // what readDir returned previously vs now by comparing full paths.
    const existingPaths = new Set(
      targetNode.children.map((c) => (c.nodeType === NodeType.Dir ? c.absolutePath : c.path)),
    );

    const newChildren = freshChildren.filter((c) => {
      const p = c.nodeType === NodeType.Dir ? c.absolutePath : c.path;
      return !existingPaths.has(p);
    });

    if (newChildren.length === 0) return;

    targetNode.extend(newChildren);
    targetNode.updateCount();
    targetNode.propagateCount();

    if (targetNode.visual?.childrenEl) {
      for (const child of newChildren) {
        renderNode(targetNode.visual.childrenEl, child);
      }
    }
  }

  // Real (user-added sample) folders

  /**
   * renders a real folder after the anchor sentinel (below all plugin folders).
   */
  async addRealFolder(folder: RealFolderDef): Promise<void> {
    const path = typeof folder === "string" ? folder : folder.path;
    const name = typeof folder === "string" ? undefined : folder.name;
    const icon = typeof folder === "string" ? undefined : (folder.icon ?? undefined);

    const node = VFSNode.real(path, name);
    const children = await this.readDir(path);

    node.extend(children);
    this.root.add(node);

    if (node.nodeType === NodeType.Dir) {
      renderNode(this.container, node, icon);
      await prefetchCount(node);
    }
  }

  // Helpers

  // inside FileTree, replace the private method body
  private readDir(path: string) {
    return readDir(path);
  }
}

/**
 * walks the VFS tree toward absolutePath, loading unvisited nodes on the way.
 * returns the node whose absolutePath matches exactly, or null if not reachable.
 */
async function findOrLoadNode(
  root: VFSNodeType,
  absolutePath: string,
): Promise<VFSNodeType | null> {
  if (root.absolutePath === absolutePath) return root;

  let match = root.children.find(
    (c) =>
      c.nodeType === NodeType.Dir &&
      (absolutePath.startsWith(c.absolutePath + SEPARATOR) || c.absolutePath === absolutePath),
  ) as VFSNodeType | undefined;

  if (!match) {
    const fresh = await readDir(root.absolutePath);

    const existingPaths = new Set(
      root.children
        .filter((c) => c.nodeType === NodeType.Dir)
        .map((c) => (c as VFSNodeType).absolutePath),
    );
    const newDirs = fresh.filter(
      (c) =>
        c.nodeType === NodeType.Dir && !existingPaths.has((c as VFSNodeType).absolutePath),
    );
    root.extend(newDirs);

    if (root.visual?.childrenEl) {
      for (const child of newDirs) {
        renderNode(root.visual.childrenEl, child);
      }
    }

    match = root.children.find(
      (c) =>
        c.nodeType === NodeType.Dir &&
        (absolutePath.startsWith(c.absolutePath + SEPARATOR) ||
          c.absolutePath === absolutePath),
    ) as VFSNodeType | undefined;
  }

  if (!match) return null;

  // Don't call loadNode here — we only need to read one level for traversal,
  // not populate the full subtree into the VFS. Read directly instead.
  if (!match.loaded) {
    const children = await readDir(match.absolutePath);
    // Only add dirs needed for further traversal, not all files.
    // This avoids polluting node.children and breaking refreshPluginFolder's diff.
    const existingPaths = new Set(
      match.children
        .filter((c) => c.nodeType === NodeType.Dir)
        .map((c) => (c as VFSNodeType).absolutePath),
    );
    const newDirs = children.filter(
      (c) =>
        c.nodeType === NodeType.Dir && !existingPaths.has((c as VFSNodeType).absolutePath),
    );
    match.extend(newDirs);
  }

  return await findOrLoadNode(match, absolutePath);
}

async function readDir(path: string): Promise<VFSChild[]> {
  const res = await invoke(IPC.ReadDir, path);
  return res
    .split("\n")
    .filter((e) => e)
    .map((p) => parseVFS(path, p));
}
