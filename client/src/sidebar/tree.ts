import { $el } from "../alias";
import { invoke, IPC } from "../invoke/invoke";
import { parseVFS } from "./parse";
import { loadNode, prefetchCount } from "./lazy-load";
import { renderNode } from "./render";
import { VFSNode, type VFSChild, type VFSNode as VFSNodeType } from "./vfs";
import { NodeType } from "./sidebar";
import { basename } from "../helpers";

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

    const freshChildren = await this.readDir(targetNode.absolutePath);

    const existingNames = new Set(
      targetNode.children.map((c) =>
        c.nodeType === NodeType.Dir ? c.name : basename(c.path),
      ),
    );

    const newChildren = freshChildren.filter(
      (c) => !existingNames.has(c.nodeType === NodeType.Dir ? c.name : basename(c.path)),
    );

    if (newChildren.length === 0) return;

    targetNode.extend(newChildren);
    targetNode.updateCount();
    targetNode.propagateCount();

    // Only render into the DOM if this node has already been expanded (has a
    // bound visual with a childrenEl).
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

  // try to find a child whose path is a prefix of the target.
  let match = root.children.find(
    (c) =>
      c.nodeType === NodeType.Dir &&
      (absolutePath.startsWith(c.absolutePath + "/") || c.absolutePath === absolutePath),
  ) as VFSNodeType | undefined;

  // no match — this directory may have new folders created since last read.
  // re-read it and merge any new children in.

  if (!match) {
    const fresh = await readDir(root.absolutePath);
    const existingNames = new Set(
      root.children.map((c) => (c.nodeType === NodeType.Dir ? c.name : basename(c.path))),
    );
    const newChildren = fresh.filter(
      (c) => !existingNames.has(c.nodeType === NodeType.Dir ? c.name : basename(c.path)),
    );
    root.extend(newChildren);

    if (root.visual?.childrenEl) {
      for (const child of newChildren) {
        renderNode(root.visual.childrenEl, child);
      }
    }

    match = root.children.find(
      (c) =>
        c.nodeType === NodeType.Dir &&
        (absolutePath.startsWith(c.absolutePath + "/") || c.absolutePath === absolutePath),
    ) as VFSNodeType | undefined;
  }

  if (!match) return null;

  if (!match.loaded) await loadNode(match);
  return await findOrLoadNode(match, absolutePath);
}

async function readDir(path: string): Promise<VFSChild[]> {
  const res = await invoke(IPC.ReadDir, path);
  return res
    .split("\n")
    .filter((e) => e)
    .map((p) => parseVFS(path, p));
}
