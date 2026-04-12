import { type VFSChild, VFSFile, VFSNode } from "./vfs";

const BYTE_OFFSET = 32;

export function parseVFS(payload: string): VFSChild {
  console.log(payload);
  const itemType = payload.charCodeAt(0) - BYTE_OFFSET;
  const isDir = itemType === 0;

  const path = payload.slice(1);
  return isDir ? VFSNode.child(path) : new VFSFile(path, itemType);
}
