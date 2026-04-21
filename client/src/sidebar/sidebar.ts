import { ONCLICK } from "../alias";
import { displayPreview } from "../browse/browse";
import { basename } from "../helpers";
import { parseVFS } from "./parse";
import { renderNode } from "./render";
import { NodeType, type VFSChild, VFSNode } from "./vfs";

declare const sidebar: HTMLDivElement;
declare const add_folder: HTMLButtonElement;

const root = VFSNode.root("__root__");

invoke("start_sample_scan");
invoke("get_sample_folders").then(async (res) => {
  const folders: string[] = res.split("\n").filter((e) => e);

  for (const folder of folders) {
    const node = VFSNode.root(folder);

    const children: VFSChild[] = await invoke("read_dir", folder).then((res) =>
      res
        .split("\n")
        .filter((e) => e)
        .map((p) => parseVFS(folder, p)),
    );

    node.extend(children);
    root.add(node);
  }

  for (const child of root.children) {
    if (child.nodeType === NodeType) {
      renderNode(sidebar, child);
    }
  }
});

sidebar[ONCLICK] = async (e) => {
  const url = (e.target as HTMLElement)?.dataset.path;
  if (!url) return;

  const path = decodeURI(url);
  const text = (await invoke("tag_path", path)) ?? "";
  const tags = text ? text.split(",") : [];

  displayPreview(path, basename(path), tags);
};

add_folder[ONCLICK] = async () => {
  const folder = await invoke("open_folder");
  const isOk = await invoke("add_sample_folder", folder);
  if (isOk !== "Ok") return;

  const node = VFSNode.root(folder);
  root.add(node);
  renderNode(sidebar, node);
};
