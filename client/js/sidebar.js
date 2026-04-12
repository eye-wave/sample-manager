class DirNode {
  children = [];
  name = "Root";
  path = "/";

  contentNode = null;
  textNode = null;

  constructor(dpath, node) {
    this.contentNode = node;

    try {
      const path = dpath.replace(/\\/g, "/");
      const [, name] = path.match(/\/([^\/]*$)/) ?? [];

      this.path = path;
      this.name = name;
    } catch {}
  }

  count() {
    return this.children.reduce(
      (sum, n) => sum + (typeof n === "string" ? 1 : n.count()),
      0,
    );
  }

  add(node) {
    this.children.push(node);
  }

  extend(nodes) {
    for (const node of nodes) {
      this.children.push(node);
    }
  }
}

(async () => {
  const sidebar = document.querySelector(".sidebar-scroll");

  const root = new DirNode(sidebar);
  const folders = await invoke("get_sample_folders").then((res) =>
    res.split("\n").filter(Boolean),
  );

  for (const folder of folders) {
    const children = await invoke("read_dir", folder).then((res) =>
      res

        .split("\n")
        .filter(Boolean)
        .map((line) => {
          const isDir = line.charAt(0) === "1";
          const path = line.slice(1);

          if (isDir) return new DirNode(path);
          return path;
        }),
    );

    const node = new DirNode(folder);
    node.extend(children);

    root.add(node);
  }

  const SIDEBAR_FOLDER = (isFolder, name, count) => `<div class="tree-section">
    <div class="tree-label">
      ${isFolder ? '<span class="tree-arrow">▶</span>' : ""}
      <span class="tree-icon">${isFolder ? "📂" : "🎵"}</span>
      <span class="tree-name">${name}</span>
      ${isFolder ? `<span class="tree-count">${count}</span>` : ""}
    </div>
  </div>`;

  function createItem(parent, isDir, name, count) {
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

  for (const node of root.children) {
    createItem(sidebar, true, node.name, node.count());
  }

  const btn = document.querySelector(".add-folder");
  console.log(btn);
  btn.addEventListener("click", async () => {
    const folder = await invoke("open_folder");
    const isOk = await invoke("add_sample_folder", folder);

    if (isOk !== "Ok") return;
    console.log("Added ", folder);
  });
})();
