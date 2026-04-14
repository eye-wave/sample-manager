import { basename, escapeHTML } from "../helpers";
import { POOL_SIZE } from "./browse";
import type { BrowseRow } from "./row";

export class TagInput {
  private tags: string[] = [];

  constructor(
    private readonly input: HTMLInputElement,
    private readonly container: HTMLElement,
    private readonly rowPool: BrowseRow[],
  ) {
    this.mount();
  }

  private mount() {
    this.input.onkeydown = (e) => {
      if (e.key === "Enter") {
        const words = this.input.value.split(/\s+/);

        words.forEach((w) => {
          if (this.tags.includes(w)) {
            return;
          }

          this.addTag(w);
        });

        this.input.value = "";
      } else if (e.key === "Backspace" && this.input.value.length === 0) {
      }
    };

    this.input.oninput = async () => {
      const query = this.input.value;
      const pool = this.rowPool;

      if (query.length === 0) {
        pool.forEach((item) => {
          item.hide();
        });

        return;
      }

      const text = await invoke(
        "search_for_sample",
        this.tags.reduce((q, t) => q + t + ",", "") + query,
      );

      const lines = text.split("\n").filter(Boolean);

      for (let i = 0; i < POOL_SIZE; i++) {
        if (i < lines.length) {
          const path = lines[i];

          pool[i].update(basename(path), null, false, []);
          pool[i].setPath(path);
        } else {
          pool[i].hide();
        }
      }
    };
  }

  private addTag(tag: string) {
    this.tags.push(tag);
    const item = document.createElement("span");
    const label = escapeHTML(tag);

    item.className = "tag-chip";
    item.innerHTML = /* HTML */ `${label}<span class="chip-x">x</span>`;

    item.onclick = () => this.removeTag(+(item.dataset.i ?? 0));

    if (this.container.firstChild) {
      this.container.insertBefore(item, this.container.firstChild);
    } else {
      this.container.appendChild(item);
    }
  }

  private removeTag(idx: number) {
    this.tags.splice(idx, 1);
    this.container.children.item(idx)?.remove();

    for (let i = this.tags.length - 1; i >= 0; i--) {
      const span = this.container.children.item(i) as HTMLSpanElement;
      if (span) span.dataset.i = `${i}`;
    }
  }
}
