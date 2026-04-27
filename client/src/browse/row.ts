import { $el, APPEND_CHILD, ONCLICK, QUERY_SELECTOR, txt } from "../alias";
import { renderTags } from "../helpers";

export type BrowseRow = ReturnType<typeof BrowseRow>;
export const BrowseRow = (
  idx: number,
  onSelect?: (i: number, p: string) => void,
  onDrag?: (p: string) => void,
  onLike?: () => void,
) => {
  const el = $el("div");
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

  const favElText = txt();
  const labelText = txt();
  const typeEl = txt();
  const bpmEl = txt();

  const labelEl = el[QUERY_SELECTOR](".item-label") as HTMLSpanElement;
  const favEl = el[QUERY_SELECTOR](".item-fav") as HTMLSpanElement;
  const tagsEl = el[QUERY_SELECTOR](".item-tags") as HTMLDivElement;

  labelEl[APPEND_CHILD](labelText);
  el[QUERY_SELECTOR](".item-type")?.[APPEND_CHILD](typeEl);
  el[QUERY_SELECTOR](".item-bpm")?.[APPEND_CHILD](bpmEl);

  favEl?.[APPEND_CHILD](favElText);

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
    bpmEl.nodeValue = bpm ? (bpm as unknown as string) : "-";

    renderTags(tagsEl, tags);

    el.style.display = "";
  };

  el[ONCLICK] = () => path && onSelect?.(idx, path);
  el.draggable = true;
  el.ondragstart = () => path && onDrag?.(path);

  const highlight = (on: boolean) => el.classList[on ? "add" : "remove"]("highlight");
  const hide = () => {
    el.style.display = "none";
    highlight(false);
  };

  favEl[ONCLICK] = (e) => {
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
