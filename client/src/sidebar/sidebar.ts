import { ONCLICK } from "../alias";
import { basename } from "../helpers";
import { playerHandle } from "../player/player";
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

  playerHandle.startPlaying(path, basename(path));
};

add_folder[ONCLICK] = async () => {
  const folder = await invoke("open_folder");
  await invoke("add_sample_folder", folder);

  const node = VFSNode.root(folder);
  root.add(node);
  renderNode(sidebar, node);
};
