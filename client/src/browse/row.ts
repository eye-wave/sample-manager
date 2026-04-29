import { $el, txt } from "../alias";
import { renderTags, setLikedView } from "../helpers";

export type BrowseRow = ReturnType<typeof BrowseRow>;
export const BrowseRow = (
  idx: number,
  onSelect?: (i: number, p: string) => void,
  onDrag?: (p: string) => void,
  onLike?: (p: string, s: boolean) => void,
) => {
  const el = $el("div");
  el.className = "list-item hidden";

  el.innerHTML = /* HTML */ `
    <div class="item-name">
      <span class="fav"></span>
      <span class="item-label"></span>
    </div>
    <div class="item-type"></div>
    <div class="item-bpm"></div>
    <div class="item-tags"></div>
  `;

  const labelText = txt();
  const typeEl = txt();
  const bpmEl = txt();

  const labelEl = el.querySelector(".item-label") as HTMLSpanElement;
  const favEl = el.querySelector(".fav") as HTMLSpanElement;
  const tagsEl = el.querySelector(".item-tags") as HTMLDivElement;

  labelEl.appendChild(labelText);
  el.querySelector(".item-type")?.appendChild(typeEl);
  el.querySelector(".item-bpm")?.appendChild(bpmEl);

  let isLiked = false;
  let path: string | null = null;

  let storedName = "";
  let storedTags: string[] = [];

  const update = (name: string, bpm: number | null, liked: boolean, tags: string[] = []) => {
    isLiked = liked;
    setLikedView(liked, favEl);

    storedName = name;
    storedTags = tags;

    labelEl.setAttribute("title", name);

    labelText.nodeValue = name;
    typeEl.nodeValue = bpm ? "Loop" : "One-shot";
    bpmEl.nodeValue = bpm ? (bpm as unknown as string) : "-";

    renderTags(tagsEl, tags);

    el.style.display = "";
  };

  el.onclick = () => path && onSelect?.(idx, path);
  el.draggable = true;
  el.ondragstart = () => path && onDrag?.(path);

  const highlight = (on: boolean) => el.classList[on ? "add" : "remove"]("highlight");
  const hide = () => {
    el.style.display = "none";
    highlight(false);
  };

  favEl.onclick = (e) => {
    if (!path) return;

    onLike?.(path, !isLiked);
    isLiked = !isLiked;

    setLikedView(isLiked, favEl);

    e.stopPropagation();
  };

  hide();

  return {
    get name() {
      return storedName;
    },
    get tags() {
      return storedTags;
    },
    el,
    favEl,
    update,
    highlight,
    hide,
    setPath(p: string) {
      path = p;
    },
    get path() {
      return path;
    },
    get isFav() {
      return isLiked;
    },
  };
};
