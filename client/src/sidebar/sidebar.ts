import { parseVFS } from "./parse";
import { renderNode } from "./render";
import { NodeType, type VFSChild, VFSNode } from "./vfs";

declare const sidebar: HTMLDivElement;
declare const add_folder: HTMLButtonElement;

const root = VFSNode.root("__root__");

const folders: string[] = await invoke("get_sample_folders").then((res) =>
  res.split("\n").filter(Boolean),
);

for (const folder of folders) {
  const node = VFSNode.root(folder);

  const children: VFSChild[] = await invoke("read_dir", folder).then((res) =>
    res.split("\n").filter(Boolean).map(parseVFS),
  );

  node.extend(children);
  root.add(node);
}

for (const child of root.children) {
  if (child.nodeType === NodeType) {
    renderNode(sidebar, child);
  }
}

add_folder.onclick = async () => {
  const folder = await invoke("open_folder");
  const isOk = await invoke("add_sample_folder", folder);
  if (isOk !== "Ok") return;

  const node = VFSNode.root(folder);
  root.add(node);
  renderNode(sidebar, node);
};
