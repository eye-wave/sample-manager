(() => {
  const browser = document.querySelector(".list-scroll");
  if (!browser) return;

  const LIST_ITEM = (name, bpm, liked, tags) => `<div class="list-item">
    <div class="item-name">
      <span class="item-fav ${liked ? "liked" : ""}">${liked ? "♥" : "♡"}</span
      ><span class="item-label">${name}</span>
    </div>
    <div class="item-type">${bpm ? "Loop" : "One-shot"}</div>
    <div class="item-bpm">${bpm || "-"}</div>
    <div class="item-tags">${tags.map((t) => `<span class="tag">${t}</span>`).join("")}</div>
  </div>`;

  const samples = Array.from({ length: 15 }, (_, i) => [
    "Lorem ipsum", //name
    Math.random() > 0.8 && 100, //bpm
    Math.random() > 0.6, //liked
    ["EDM"], //tags
  ]);

  browser.insertAdjacentHTML(
    "beforeend",
    samples.map((sample) => LIST_ITEM(...sample)).join(""),
  );
})();
