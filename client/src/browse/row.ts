export type BrowseRow = ReturnType<typeof BrowseRow>;
export const BrowseRow = (onSelect?: (p: string) => void, onLike?: () => void) => {
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
  const labelEl = document.createTextNode("");
  const typeEl = document.createTextNode("");
  const bpmEl = document.createTextNode("");

  const favEl = el.querySelector(".item-fav") as HTMLSpanElement;
  const tagsEl = el.querySelector(".item-tags") as HTMLDivElement;

  el.querySelector(".item-label")?.appendChild(labelEl);
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

  const update = (name: string, bpm: number | null, liked: boolean, tags: string[] = []) => {
    setLiked(liked);

    labelEl.nodeValue = name;
    typeEl.nodeValue = bpm ? "Loop" : "One-shot";
    bpmEl.nodeValue = bpm ? String(bpm) : "-";

    const joined = tags.join(",");
    if (tagsEl.dataset.tags !== joined) {
      tagsEl.dataset.tags = joined;
      tagsEl.innerHTML = tags.map((t) => /* HTML */ `<span class="tag">${t}</span>`).join("");
    }

    el.style.display = "";
  };

  const hide = () => {
    el.style.display = "none";
  };

  el.onclick = () => path && onSelect?.(path);

  favEl.onclick = (e) => {
    setLiked(!isLiked);
    onLike?.();
    e.stopPropagation();
  };

  hide();

  return {
    el,
    update,
    hide,
    setPath,
    setLiked,
  };
};
