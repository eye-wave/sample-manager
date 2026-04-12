declare const list_scroll: HTMLDivElement;

type Sample = {
  name: string;
  bpm: number | null;
  liked: boolean;
  tags: string[];
};

const tagEl = (tags: string[] = []) =>
  tags.map((t) => `<span class="tag">${t}</span>`).join("");

const LIST_ITEM = ({ name, bpm, liked, tags }: Sample) =>
  /* HTML */ `<div class="list-item">
    <div class="item-name">
      <span class="item-fav ${liked ? "liked" : ""}">${liked ? "♥" : "♡"}</span
      ><span class="item-label">${name}</span>
    </div>
    <div class="item-type">${bpm ? "Loop" : "One-shot"}</div>
    <div class="item-bpm">${bpm || "-"}</div>
    <div class="item-tags">${tagEl(tags)}</div>
  </div>`;

const samples: Sample[] = Array.from({ length: 15 }, (_, i) => ({
  name: "Lorem ipsum",
  bpm: Math.random() > 0.8 ? 100 : null,
  liked: Math.random() > 0.6,
  tags: ["EDM"],
}));

list_scroll.insertAdjacentHTML(
  "beforeend",
  samples.map((sample) => LIST_ITEM(sample)).join(""),
);
