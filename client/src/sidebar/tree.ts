import { $el } from "../alias";
import { SEPARATOR } from "../helpers";
import { IPC, invoke } from "../invoke/invoke";
import { parseVFS } from "./parse";
import { prefetchCount, renderNode } from "./render";
import { NodeType } from "./sidebar";
import { type VFSChild, VFSNode, type VFSNode as VFSNodeType } from "./vfs";

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

// ---------------------------------------------------------------------------
// VFS helpers — previously split between lazy-load.ts and tree.ts
// ---------------------------------------------------------------------------

async function readDir(path: string): Promise<VFSChild[]> {
  const res = await invoke(IPC.ReadDir, path);
  return res
    .split("\n")
    .filter(Boolean)
    .map((p) => parseVFS(path, p));
}

/** Walks the VFS toward `absolutePath`, loading unseen nodes on the way. */
async function findOrLoadNode(
  root: VFSNodeType,
  absolutePath: string,
): Promise<VFSNodeType | null> {
  if (root.absolutePath === absolutePath) return root;

  const isAncestor = (c: VFSChild) =>
    c.nodeType === NodeType.Dir &&
    (absolutePath.startsWith(c.absolutePath + SEPARATOR) || c.absolutePath === absolutePath);

  let match = root.children.find(isAncestor) as VFSNodeType | undefined;

  if (!match) {
    const fresh = await readDir(root.absolutePath);
    const existing = new Set(
      root.children
        .filter((c): c is VFSNodeType => c.nodeType === NodeType.Dir)
        .map((c) => c.absolutePath),
    );
    const newDirs = fresh.filter(
      (c): c is VFSNodeType => c.nodeType === NodeType.Dir && !existing.has(c.absolutePath),
    );
    root.extend(newDirs);
    if (root.visual?.childrenEl) {
      for (const child of newDirs) renderNode(root.visual.childrenEl, child);
    }
    match = root.children.find(isAncestor) as VFSNodeType | undefined;
  }

  if (!match) return null;

  if (!match.loaded) {
    const children = await readDir(match.absolutePath);
    const existing = new Set(
      match.children
        .filter((c): c is VFSNodeType => c.nodeType === NodeType.Dir)
        .map((c) => c.absolutePath),
    );
    const newDirs = children.filter(
      (c): c is VFSNodeType => c.nodeType === NodeType.Dir && !existing.has(c.absolutePath),
    );
    match.extend(newDirs);
  }

  return findOrLoadNode(match, absolutePath);
}

/** Loads a node's children into the VFS and DOM (previously in lazy-load.ts). */
export async function loadNode(node: VFSNodeType): Promise<void> {
  if (node.loaded) return;
  node.loaded = true;

  const fresh = await readDir(node.path());
  const existing = new Set(
    node.children.map((c) => (c.nodeType === NodeType.Dir ? c.absolutePath : c.path)),
  );
  const newChildren = fresh.filter((c) => {
    const p = c.nodeType === NodeType.Dir ? c.absolutePath : c.path;
    return !existing.has(p);
  });

  node.extend(newChildren);
  node.updateCount();
  node.propagateCount();

  if (node.visual?.childrenEl) {
    for (const child of node.children) renderNode(node.visual.childrenEl, child);
  }
}

// ---------------------------------------------------------------------------
// FileTree
// ---------------------------------------------------------------------------

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

  async addPluginFolder(folder: PluginFolderDef): Promise<void> {
    const { path } = folder;
    if (this.pluginEntries.has(path)) return;

    const node = VFSNode.plugin(path, folder.name);
    const children = await readDir(path);
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

  removePluginFolder(path: string): void {
    const entry = this.pluginEntries.get(path);
    if (!entry) return;
    entry.sectionEl.remove();
    entry.childrenEl?.remove();
    this.pluginEntries.delete(path);
  }

  async refreshPluginFolder(pluginPath: string, changedSubpath?: string): Promise<void> {
    const entry = this.pluginEntries.get(pluginPath);
    if (!entry) return;

    const targetNode = changedSubpath
      ? await findOrLoadNode(entry.node, changedSubpath)
      : entry.node;

    if (!targetNode) return;

    const freshChildren = await readDir(targetNode.absolutePath);
    const existing = new Set(
      targetNode.children.map((c) => (c.nodeType === NodeType.Dir ? c.absolutePath : c.path)),
    );
    const newChildren = freshChildren.filter((c) => {
      const p = c.nodeType === NodeType.Dir ? c.absolutePath : c.path;
      return !existing.has(p);
    });

    if (newChildren.length === 0) return;

    targetNode.extend(newChildren);
    targetNode.updateCount();
    targetNode.propagateCount();

    if (targetNode.visual?.childrenEl) {
      for (const child of newChildren) renderNode(targetNode.visual.childrenEl, child);
    }
  }

  async addRealFolder(folder: RealFolderDef): Promise<void> {
    const path = typeof folder === "string" ? folder : folder.path;
    const name = typeof folder === "string" ? undefined : folder.name;
    const icon = typeof folder === "string" ? undefined : (folder.icon ?? undefined);

    const node = VFSNode.real(path, name);
    const children = await readDir(path);
    node.extend(children);
    this.root.add(node);

    if (node.nodeType === NodeType.Dir) {
      renderNode(this.container, node, icon);
      await prefetchCount(node);
    }
  }
}
