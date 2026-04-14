export const FOLDER_CLOSED = "📁";
export const FOLDER_OPEN = "📂";

export const SIDEBAR_FOLDER = (name: string) =>
  /* HTML */ `<div class="tree-section">
    <div class="tree-label">
      <span class="tree-arrow">▶</span>
      <span class="tree-icon"></span>
      <span class="tree-name">${name}</span>
      <span class="tree-count"></span>
    </div>
  </div>`;

const ICON_OTHER = 255;
const itemIcons: Record<number, string> = { 1: "♪", 2: "🎹", 3: "🛠️", [ICON_OTHER]: "📄" };

export const SIDEBAR_ITEM = (name: string, ftype: number) =>
  /* HTML */ `<div class="tree-section item">${itemIcons[ftype] ?? itemIcons[ICON_OTHER]} ${name}</div>`;
