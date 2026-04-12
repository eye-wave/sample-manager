const folderArrow = (isFolder: boolean) =>
  isFolder ? '<span class="tree-arrow">▶</span>' : "";

const folderEl = (isFolder: boolean) => (isFolder ? `<span class="tree-count"></span>` : "");

export const SIDEBAR_ITEM = (isFolder: boolean, name: string) =>
  /* HTML */ ` <div class="tree-section">
    <div class="tree-label">
      ${folderArrow(isFolder)}
      <span class="tree-icon">${isFolder ? "📂" : "🎵"}</span>
      <span class="tree-name">${name}</span>
      ${folderEl(isFolder)}
    </div>
  </div>`;
