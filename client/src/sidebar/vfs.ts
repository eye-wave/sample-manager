import { txt } from "../alias";
import { basename } from "../helpers";
import { NodeKind, NodeType } from "./sidebar";

const FOLDER_CLOSED =
  "M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z";

const FOLDER_OPEN =
  "m6 14 1.5-2.9A2 2 0 0 1 9.24 10H20a2 2 0 0 1 1.94 2.5l-1.54 6a2 2 0 0 1-1.95 1.5H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.69.9l.81 1.2a2 2 0 0 0 1.67.9H18a2 2 0 0 1 2 2v2";

export type VFSChild = VFSFile | VFSNode;

type VFSFile = {
  nodeType: NodeType.File;
  path: string;
  ftype: number;
};

type VFSVisual = ReturnType<typeof createVisualNode>;

const createVisualNode = (section: Element) => {
  const labelEl = section.querySelector<HTMLElement>(".tree-label");
  const childrenEl = section.nextElementSibling as HTMLElement | null;
  const arrowClassList = section.querySelector(".tree-arrow")?.classList ?? null;
  const countSpan = section.querySelector<HTMLElement>(".tree-count");
  const countEl = countSpan ? txt() : null;
  if (countSpan && countEl) countSpan.appendChild(countEl);

  // biome-ignore lint/style/noNonNullAssertion: trust
  const iconEl = section.querySelector<SVGPathElement>("[data-folder] path")!;
  iconEl.setAttribute("d", FOLDER_CLOSED);

  return {
    labelEl,
    childrenEl,
    arrowClassList,
    countEl,
    iconEl,

    updateCount(count: number | null = null) {
      if (countEl) countEl.nodeValue = (count != null ? count : "") as string;
    },

    toggle() {
      arrowClassList?.toggle("open");
      const open = arrowClassList?.contains("open");
      iconEl?.setAttribute("d", open ? FOLDER_OPEN : FOLDER_CLOSED);
      if (childrenEl) {
        childrenEl.style.display = open ? "block" : "none";
      }
    },
  };
};

export type VFSNode = {
  nodeType: typeof NodeType.Dir;
  kind: NodeKind;
  /** Basename of this node, used for display and path reconstruction. */
  name: string;
  /**
   * Full absolute path. Set explicitly on all nodes so path-based lookups
   * (e.g. findOrLoadNode) work without relying on parent-chain reconstruction.
   */
  absolutePath: string;
  parent: null | VFSNode;
  visual: null | VFSVisual;
  loaded: boolean;
  displayName: string;
  children: VFSChild[];
  //
  path: () => string;
  count: () => null | number;
  add: (child: VFSChild) => void;
  extend: (children: VFSChild[]) => void;
  bind: (section: Element) => void;
  updateCount: () => void;
  propagateCount: () => void;
  toggle: () => void;
};

export const VFSNode = {
  /** Invisible logical root — never rendered. */
  root(path: string, name?: string): VFSNode {
    const n = createNode(path, path, NodeKind.Root);
    n.displayName = name ?? basename(path);
    return n;
  },

  /** A user-added sample folder rendered in the lower section. */
  real(path: string, name?: string): VFSNode {
    const n = createNode(basename(path), path, NodeKind.Real);
    n.displayName = name ?? basename(path);
    return n;
  },

  /** A plugin folder rendered in the upper section. */
  plugin(path: string, name: string): VFSNode {
    const n = createNode(basename(path), path, NodeKind.Plugin);
    n.displayName = name;
    return n;
  },

  /**
   * A lazily-discovered child directory inside any folder.
   * segment is the full absolute path as produced by joinPath in parseVFS.
   */
  child(segment: string): VFSNode {
    const name = basename(segment);
    const n = createNode(name, segment, NodeKind.Real);
    n.displayName = name;
    return n;
  },

  file(path: string, ftype: number): VFSFile {
    return { nodeType: NodeType.File, ftype, path };
  },
};

const createNode = (name: string, absolutePath: string, kind: NodeKind): VFSNode => {
  const node: VFSNode = {
    nodeType: NodeType.Dir,
    kind,
    name,
    absolutePath,
    parent: null,
    visual: null,
    loaded: false,
    displayName: "",
    children: [],

    path() {
      return node.absolutePath;
    },

    count() {
      const total = node.children.reduce(
        (s: number, c: VFSChild) =>
          c.nodeType === NodeType.File ? s + 1 : s + (c.count() ?? 0),
        0,
      );
      return node.loaded && total === 0 ? 0 : total === 0 ? null : total;
    },

    add(child) {
      if (child.nodeType === NodeType.Dir) child.parent = node;
      node.children.push(child);
    },

    extend(children) {
      for (const c of children) node.add(c);
    },

    bind(section: Element) {
      node.visual = createVisualNode(section);
    },

    updateCount() {
      node.visual?.updateCount(node.count());
    },

    propagateCount() {
      let n = node;
      while (n.parent) {
        n = n.parent;
        n.updateCount();
      }
    },

    toggle() {
      node.visual?.toggle();
    },
  };

  return node;
};
