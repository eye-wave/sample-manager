import { joinPath } from "../helpers";
import { type VFSChild, VFSNode } from "./vfs";

const BYTE_OFFSET = 32;

export function parseVFS(basepath: string, payload: string): VFSChild {
  const itemType = payload.charCodeAt(0) - BYTE_OFFSET;
  const isDir = itemType === 0;

  const name = payload.slice(1);
  const path = joinPath(basepath, name);

  return isDir ? VFSNode.child(path) : VFSNode.file(path, itemType);
}
