(async () => {
  const sidebar = document.querySelector(".sidebar-scroll");
  if (!sidebar) return;

  const response = (await invoke("search_path", "/home/eyewave/Music")).split(
    "\n",
  );

  const folders = await invoke("get_sample_folders").then((res) =>
    res.split("\n").filter(Boolean),
  );

  const SIDEBAR_FOLDER = (isFolder, name, count) => `<div class="tree-section">
    <div class="tree-label">
      ${isFolder ? '<span class="tree-arrow">▶</span>' : ""}
      <span class="tree-icon">${isFolder ? "📂" : "🎵"}</span>
      <span class="tree-name">${name}</span>
      ${isFolder ? `<span class="tree-count">${count}</span>` : ""}
    </div>
  </div>`;

  function createItem(parent, line) {
    const isDir = line.charAt(0) === "1";
    const [name, count] = line.slice(1).split(":");

    parent.insertAdjacentHTML(
      "beforeend",
      SIDEBAR_FOLDER(isDir, name, count ?? "?"),
    );

    if (isDir) {
      parent.insertAdjacentHTML(
        "beforeend",
        '<div class="tree-children"></div>',
      );
    }
  }

  response.forEach((i) => createItem(sidebar, i));

  const btn = document.querySelector(".add-folder");
  btn.onclick = async () => {
    const folder = await invoke("open_folder");
    const isOk = await invoke("add_sample_folder", folder);

    console.log(isOk);
  };
})();
