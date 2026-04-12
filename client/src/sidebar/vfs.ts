export type VFSChild = VFSNode | string;

export class VFSNode {
  path: string;
  name: string;
  children: VFSChild[] = [];
  parent: VFSNode | null = null;
  loaded: boolean = false;

  labelEl: HTMLElement | null = null;
  countEl: Text | null = null;
  childrenEl: HTMLElement | null = null;

  constructor(path: string) {
    this.path = path;
    this.name = path.split(/[\\/]/).filter(Boolean).at(-1) ?? path;
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
