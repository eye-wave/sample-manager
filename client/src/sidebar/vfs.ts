import { APPEND_CHILD, QUERY_SELECTOR, txt } from "../alias";
import { basename } from "../helpers";

const FOLDER_CLOSED =
  "M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z";

const FOLDER_OPEN =
  "m6 14 1.5-2.9A2 2 0 0 1 9.24 10H20a2 2 0 0 1 1.94 2.5l-1.54 6a2 2 0 0 1-1.95 1.5H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.69.9l.81 1.2a2 2 0 0 0 1.67.9H18a2 2 0 0 1 2 2v2";

export const FileType = 0 as const;
export const NodeType = 1 as const;

export type VFSChild = VFSFile | VFSNode;

export interface VFSCNodeType {
  readonly nodeType: number;
}

type VFSFile = {
  nodeType: typeof FileType;
  path: string;
  ftype: number;
};

type VFSVisual = ReturnType<typeof createVisualNode>;
const createVisualNode = (section: Element) => {
  const labelEl = section[QUERY_SELECTOR]<HTMLElement>(".tree-label");
  const childrenEl = section.nextElementSibling as HTMLElement | null;

  const arrowClassList = section[QUERY_SELECTOR](".tree-arrow")?.classList ?? null;

  const countSpan = section[QUERY_SELECTOR]<HTMLElement>(".tree-count");
  const countEl = countSpan ? txt() : null;
  if (countSpan && countEl) countSpan[APPEND_CHILD](countEl);

  // biome-ignore lint/style/noNonNullAssertion: trust
  const iconEl = section[QUERY_SELECTOR]<SVGPathElement>("path")!;

  iconEl.setAttribute("d", FOLDER_CLOSED);

  return {
    labelEl,
    childrenEl,
    arrowClassList,
    countEl,
    iconEl,

    updateCount(count: number | null = null) {
      if (countEl) countEl.nodeValue = count != null ? (count as unknown as string) : "";
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
  nodeType: typeof NodeType;
  name: string;
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
  root(path: string) {
    const n = createNode(path);
    n.displayName = basename(path);
    return n;
  },

  child(segment: string) {
    const name = basename(segment);
    const n = createNode(name);
    n.displayName = name;
    return n;
  },

  file(path: string, ftype: number): VFSFile {
    return { nodeType: FileType, ftype, path };
  },
};

const createNode = (name: string): VFSNode => {
  const node: VFSNode = {
    nodeType: NodeType,
    name,
    parent: null,
    visual: null,
    loaded: false,
    displayName: "",
    children: [],

    path() {
      if (!node?.parent?.parent) return node.name;

      const parts: string[] = [];
      let n = node;

      while (n.parent?.parent) {
        parts.unshift(n.name);
        n = n.parent;
      }

      return n.name + "/" + parts.join("/");
    },

    count() {
      const total = node.children.reduce(
        (s: number, c: VFSChild) =>
          c.nodeType === FileType ? s + 1 : s + (c.count() ?? FileType),
        0,
      );

      return node.loaded && total === 0 ? 0 : total === 0 ? null : total;
    },

    add(child) {
      if (child.nodeType === NodeType) child.parent = node;
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
