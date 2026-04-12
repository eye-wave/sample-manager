import { renderNode } from "./render";
import { VFSNode, type VFSChild } from "./vfs";

declare const sidebar: HTMLDivElement;
declare const add_folder: HTMLButtonElement;

const root = new VFSNode("__root__");

const folders: string[] = await invoke("get_sample_folders").then((res) =>
  res.split("\n").filter(Boolean),
);

for (const folder of folders) {
  const node = new VFSNode(folder);

  const children: VFSChild[] = await invoke("read_dir", folder).then((res) =>
    res
      .split("\n")
      .filter(Boolean)
      .map((line): VFSChild => {
        const isDir = line.charAt(0) === "1";
        const path = line.slice(1);
        return isDir ? new VFSNode(path) : path;
      }),
  );

  node.extend(children);
  root.add(node);
}

for (const child of root.children) {
  if (typeof child !== "string") {
    renderNode(sidebar, child);
  }
}

add_folder.onclick = async () => {
  const folder = await invoke("open_folder");
  const isOk = await invoke("add_sample_folder", folder);
  if (isOk !== "Ok") return;

  const node = new VFSNode(folder);
  root.add(node);
  renderNode(sidebar, node);
};
