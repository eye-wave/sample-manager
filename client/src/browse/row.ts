import { renderTags } from "../helpers";

export type BrowseRow = ReturnType<typeof BrowseRow>;
export const BrowseRow = (
  idx: number,
  onSelect?: (i: number, p: string) => void,
  onLike?: () => void,
) => {
  const el = document.createElement("div");
  el.className = "list-item hidden";

  el.innerHTML = /* HTML */ `
    <div class="item-name">
      <span class="item-fav"></span>
      <span class="item-label"></span>
    </div>
    <div class="item-type"></div>
    <div class="item-bpm"></div>
    <div class="item-tags"></div>
  `;

  const favElText = document.createTextNode("");
  const labelText = document.createTextNode("");
  const typeEl = document.createTextNode("");
  const bpmEl = document.createTextNode("");

  const labelEl = el.querySelector(".item-label") as HTMLSpanElement;
  const favEl = el.querySelector(".item-fav") as HTMLSpanElement;
  const tagsEl = el.querySelector(".item-tags") as HTMLDivElement;

  labelEl.appendChild(labelText);
  el.querySelector(".item-type")?.appendChild(typeEl);
  el.querySelector(".item-bpm")?.appendChild(bpmEl);

  favEl?.appendChild(favElText);

  let isLiked = false;
  let path: string | null = null;

  const setLiked = (liked: boolean) => {
    favElText.nodeValue = liked ? "♥" : "♡";
    if (favEl) favEl.className = `item-fav ${liked ? "liked" : ""}`;
    isLiked = liked;
  };

  const setPath = (p: string) => (path = p);

  let storedName = "";
  let storedTags: string[] = [];

  const update = (name: string, bpm: number | null, liked: boolean, tags: string[] = []) => {
    setLiked(liked);

    storedName = name;
    storedTags = tags;

    labelEl.setAttribute("title", name);

    labelText.nodeValue = name;
    typeEl.nodeValue = bpm ? "Loop" : "One-shot";
    bpmEl.nodeValue = bpm ? String(bpm) : "-";

    renderTags(tagsEl, tags);

    el.style.display = "";
  };

  el.onclick = () => path && onSelect?.(idx, path);

  const highlight = (on: boolean) => el.classList[on ? "add" : "remove"]("highlight");
  const hide = () => {
    el.style.display = "none";
    highlight(false);
  };

  favEl.onclick = (e) => {
    setLiked(!isLiked);
    onLike?.();
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
    update,
    highlight,
    hide,
    setPath,
    setLiked,
  };
};
