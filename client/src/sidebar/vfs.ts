export type VFSChild = VFSNode | string;

export class VFSNode {
  name: string;
  displayName: string = "";
  children: VFSChild[] = [];
  parent: VFSNode | null = null;
  loaded: boolean = false;

  labelEl: HTMLElement | null = null;
  countEl: Text | null = null;
  childrenEl: HTMLElement | null = null;

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
      if (typeof c === "string") return sum + 1;
      return sum + (c.count() ?? 0);
    }, 0);
    return this.loaded && total === 0 ? 0 : total === 0 ? null : total;
  }

  add(child: VFSChild): void {
    if (typeof child !== "string") child.parent = this;
    this.children.push(child);
  }

  extend(children: VFSChild[]): void {
    for (const child of children) this.add(child);
  }

  bind(section: Element): void {
    this.labelEl = section.querySelector<HTMLElement>(".tree-label");
    this.childrenEl = section.nextElementSibling as HTMLElement | null;

    const countSpan = section.querySelector<HTMLElement>(".tree-count");
    if (countSpan) {
      this.countEl = document.createTextNode("");
      countSpan.appendChild(this.countEl);
    }
  }

  updateCount(): void {
    if (this.countEl) {
      const c = this.count();
      this.countEl.nodeValue = c !== null ? String(c) : "";
    }
  }

  propagateCount(): void {
    let node: VFSNode = this;
    while (node.parent !== null) {
      node = node.parent;
      node.updateCount();
    }
  }

  toggle(): void {
    this.labelEl?.classList.toggle("open");
    if (this.childrenEl) {
      this.childrenEl.style.display = this.labelEl?.classList.contains("open")
        ? "block"
        : "none";
    }
  }
}
