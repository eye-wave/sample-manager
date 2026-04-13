export class BrowseRow {
  el: HTMLElement;

  private favElText: Text;
  private favEl: HTMLElement;
  private labelEl: Text;
  private typeEl: Text;
  private bpmEl: Text;

  tagsEl: HTMLElement;

  constructor() {
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
  }

  update(name: string, bpm: number | null, liked: boolean, tags: string[] = []) {
    this.favElText.nodeValue = liked ? "♥" : "♡";
    this.favEl.className = `item-fav ${liked ? "liked" : ""}`;
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
