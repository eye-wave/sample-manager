import { basename } from "../helpers";
import { FOLDER_CLOSED, FOLDER_OPEN } from "./template";

export const FileType = 0 as const;
export const NodeType = 1 as const;

export type VFSChild = VFSFile | VFSNode;

export interface VFSCNodeType {
  readonly nodeType: number;
}

type VFSFile = {
  nodeType: typeof FileType;
  name: string;
  ftype: number;
};

type VFSVisual = ReturnType<typeof createVisualNode>;
const createVisualNode = (section: Element) => {
  const labelEl = section.querySelector<HTMLElement>(".tree-label");
  const childrenEl = section.nextElementSibling as HTMLElement | null;

  const arrowClassList = section.querySelector(".tree-arrow")?.classList ?? null;

  const countSpan = section.querySelector<HTMLElement>(".tree-count");
  const countEl = countSpan ? document.createTextNode("") : null;
  if (countSpan && countEl) countSpan.appendChild(countEl);

  const iconSpan = section.querySelector<HTMLElement>(".tree-icon");
  const iconEl = iconSpan ? document.createTextNode(FOLDER_CLOSED) : null;
  if (iconSpan && iconEl) iconSpan.appendChild(iconEl);

  return {
    labelEl,
    childrenEl,
    arrowClassList,
    countEl,
    iconEl,

    updateCount(count: number | null = null) {
      if (countEl) countEl.nodeValue = count != null ? String(count) : "";
    },

    toggle() {
      arrowClassList?.toggle("open");

      const open = arrowClassList?.contains("open");

      if (iconEl) iconEl.nodeValue = open ? FOLDER_OPEN : FOLDER_CLOSED;
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

  file(name: string, ftype: number): VFSFile {
    return { nodeType: FileType, name, ftype };
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
