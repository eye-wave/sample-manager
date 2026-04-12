import { FOLDER_CLOSED, FOLDER_OPEN } from "./template";

export const FileType = 0 as const;
export const NodeType = 1 as const;

export type VFSChild = VFSFile | VFSNode;

export interface VFSCNodeType {
  readonly nodeType: number;
}

export class VFSFile implements VFSCNodeType {
  readonly nodeType = FileType;

  constructor(
    public name: string,
    public ftype: number,
  ) {}

  public basename(): string {
    return this.name.split(/[\\/]/).at(-1) ?? this.name;
  }
}

class VFSVisual {
  labelEl: HTMLElement | null = null;
  countEl: Text | null = null;
  iconEl: Text | null = null;

  childrenEl: HTMLElement | null = null;
  arrowClassList: DOMTokenList | null = null;

  constructor(section: Element) {
    this.labelEl = section.querySelector<HTMLElement>(".tree-label");
    this.childrenEl = section.nextElementSibling as HTMLElement | null;

    this.arrowClassList = section.querySelector(".tree-arrow")?.classList ?? null;

    const countSpan = section.querySelector<HTMLElement>(".tree-count");
    if (countSpan) {
      this.countEl = document.createTextNode("");
      countSpan.appendChild(this.countEl);
    }

    const iconSpan = section.querySelector<HTMLElement>(".tree-icon");
    if (iconSpan) {
      this.iconEl = document.createTextNode(FOLDER_CLOSED);
      iconSpan.appendChild(this.iconEl);
    }
  }

  updateCount(count: number | null = null) {
    if (this.countEl) {
      const c = count;
      this.countEl.nodeValue = c !== null ? String(c) : "";
    }
  }

  toggle() {
    this.arrowClassList?.toggle("open");

    const isToggledOn = this.arrowClassList?.contains("open");

    if (this.iconEl) this.iconEl.nodeValue = isToggledOn ? FOLDER_OPEN : FOLDER_CLOSED;
    if (this.childrenEl) this.childrenEl.style.display = isToggledOn ? "block" : "none";
  }
}

export class VFSNode implements VFSCNodeType {
  readonly nodeType = NodeType;

  private name: string;
  private parent: VFSNode | null = null;

  public visual: VFSVisual | null = null;

  public loaded: boolean = false;
  public displayName: string = "";
  public children: VFSChild[] = [];

  // Use VFSNode.root() for top-level folders, VFSNode.child() for read_dir entries.
  private constructor(name: string) {
    this.name = name;
  }

  // Preserves the full absolute path as the name — path() returns it as-is
  // since there is no parent to walk up to.
  static root(absolutePath: string): VFSNode {
    const node = new VFSNode(absolutePath);
    node.displayName = absolutePath.split(/[\\/]/).filter(Boolean).at(-1) ?? absolutePath;

    return node;
  }

  static child(segment: string): VFSNode {
    const name = segment.split(/[\\/]/).filter(Boolean).at(-1) ?? segment;

    const node = new VFSNode(name);
    node.displayName = name;

    return node;
  }

  path(): string {
    if (this.parent === null || this.parent.parent === null) {
      return this.name;
    }
    const parts: string[] = [];

    let node: VFSNode = this;
    while (node.parent !== null && node.parent.parent !== null) {
      parts.unshift(node.name);
      node = node.parent;
    }
    return node.name + "/" + parts.join("/");
  }

  count(): number | null {
    const total = this.children.reduce((sum, c) => {
      if (c.nodeType === FileType) return sum + 1;
      return sum + (c.count() ?? FileType);
    }, 0);
    return this.loaded && total === 0 ? 0 : total === 0 ? null : total;
  }

  add(child: VFSChild) {
    if (child.nodeType === NodeType) child.parent = this;
    this.children.push(child);
  }

  extend(children: VFSChild[]) {
    for (const child of children) this.add(child);
  }

  bind(section: Element) {
    this.visual = new VFSVisual(section);
  }

  updateCount() {
    this.visual?.updateCount(this.count());
  }

  propagateCount() {
    let node: VFSNode = this;
    while (node.parent !== null) {
      node = node.parent;
      node.updateCount();
    }
  }

  toggle() {
    this.visual?.toggle();
  }
}
