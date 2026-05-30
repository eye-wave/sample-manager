import { $el, txt } from "../alias";
import { renderTags, setLikedView, startDrag } from "../helpers";

export type OnSelectProps =
  | {
      type: "native";
      path: string;
    }
  | { type: "plug"; id: string; url: string; name: string };

export type BrowseRow = ReturnType<typeof BrowseRow>;
export const BrowseRow = (
  idx: number,
  onSelect?: (i: number, props: OnSelectProps) => void,
  onFavToggle?: (p: string) => void,
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

  let path: string | null = null;
  let url: string | null = null;
  let pluginId: string | null = null;

  let storedName = "";
  let storedTags: string[] = [];

  const update = (name: string, bpm: number | null, liked: boolean, tags: string[] = []) => {
    setLikedView(liked, favEl);

    storedName = name;
    storedTags = tags;

    labelEl.setAttribute("title", name);
    labelText.nodeValue = name;
    typeEl.nodeValue = bpm ? "Loop" : "One-shot";
    bpmEl.nodeValue = (bpm ? bpm : "-") as string;

    renderTags(tagsEl, tags);
    el.style.display = "";
  };

  el.onclick = () => {
    const props =
      pluginId === null
        ? { type: "native" as const, path }
        : ({
            type: "plug" as const,
            url,
            id: pluginId,
            name: storedName,
          } as Partial<OnSelectProps>);

    if (props.type === "native" && !props.path) return;
    if (props.type === "plug" && !props.url && !props.id) return;

    onSelect?.(idx, props as OnSelectProps);
  };
  el.draggable = true;
  el.ondragstart = () => path && startDrag(path);

  const highlight = (on: boolean) => el.classList[on ? "add" : "remove"]("highlight");
  const hide = () => {
    el.style.display = "none";
    highlight(false);
  };

  favEl.onclick = (e) => {
    if (!path) return;
    onFavToggle?.(path);
    e.stopPropagation();
  };

  hide();

  return {
    set name(s: string) {
      storedName = s;
    },
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
      pluginId = null;
    },
    get path() {
      return path;
    },
    setUrl(u: string, id: string) {
      url = u;
      pluginId = id;
    },
    get url() {
      return url;
    },
  };
};
