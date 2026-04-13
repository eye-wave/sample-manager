export class BrowseRow {
  el: HTMLElement;

  private favElText: Text;
  private favEl: HTMLElement;
  private labelEl: Text;
  private typeEl: Text;
  private bpmEl: Text;

  tagsEl: HTMLElement;

  private isLiked = false;
  private path: string | null = null;

  constructor(onSelect?: (path: string) => void, onLike?: () => void) {
    this.el = document.createElement("div");
    this.el.className = "list-item hidden";

    this.el.innerHTML = /* HTML */ `
      <div class="item-name">
        <span class="item-fav"></span><span class="item-label"></span>
      </div>
      <div class="item-type"></div>
      <div class="item-bpm"></div>
      <div class="item-tags"></div>
    `;

    this.favElText = document.createTextNode("");
    this.labelEl = document.createTextNode("");
    this.typeEl = document.createTextNode("");
    this.bpmEl = document.createTextNode("");

    // biome-ignore-start lint/style/noNonNullAssertion: this can't fail
    this.favEl = this.el.querySelector(".item-fav")!;
    this.el.querySelector(".item-label")!.appendChild(this.labelEl);
    this.el.querySelector(".item-type")!.appendChild(this.typeEl);
    this.el.querySelector(".item-bpm")!.appendChild(this.bpmEl);

    this.tagsEl = this.el.querySelector(".item-tags")!;
    // biome-ignore-end lint/style/noNonNullAssertion: this can't fail

    this.favEl.appendChild(this.favElText);
    this.hide();

    this.el.onclick = () => {
      this.path && onSelect?.(this.path);
    };

    this.favEl.onclick = () => {
      this.setLiked(!this.isLiked);
      onLike?.();
    };
  }

  private setLiked(liked: boolean) {
    this.favElText.nodeValue = liked ? "♥" : "♡";
    this.favEl.className = `item-fav ${liked ? "liked" : ""}`;
    this.isLiked = liked;
  }

  setPath(path: string) {
    this.path = path;
  }

  update(name: string, bpm: number | null, liked: boolean, tags: string[] = []) {
    this.setLiked(liked);
    this.labelEl.nodeValue = name;
    this.typeEl.nodeValue = bpm ? "Loop" : "One-shot";
    this.bpmEl.nodeValue = bpm ? String(bpm) : "-";

    const newTags = tags.join(",");
    if (this.tagsEl.dataset.tags !== newTags) {
      this.tagsEl.dataset.tags = newTags;
      this.tagsEl.innerHTML = tags.map((t) => `<span class="tag">${t}</span>`).join("");
    }

    this.el.style.display = "";
  }

  hide() {
    this.el.style.display = "none";
  }
}
