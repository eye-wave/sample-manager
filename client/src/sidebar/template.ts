export const SIDEBAR_FOLDER = (name: string) =>
  /* HTML */ `<div class="tree-section">
    <div class="tree-label">
      <span class="tree-arrow">▶</span>
      <span class="tree-icon">
      <svg
        width="18"
        height="18"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
      ><path></svg></span>
      <span class="tree-name">${name}</span>
      <span class="tree-count"></span>
    </div>
  </div>`;

const ICON_OTHER = 254;

const itemIcons: Record<number, string> = ["♪", "🎹", "🛠️"];
itemIcons[ICON_OTHER] = "📄";

export const SIDEBAR_ITEM = (name: string, ftype: number, fullpath: string) =>
  `<div class="tree-section item" data-path=${encodeURI(fullpath)} title="${name}">${itemIcons[ftype - 1] ?? itemIcons[ICON_OTHER]} ${name}</div>`;
