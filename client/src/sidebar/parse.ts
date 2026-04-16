import { type VFSChild, VFSNode } from "./vfs";

const BYTE_OFFSET = 32;

export function parseVFS(payload: string): VFSChild {
  const itemType = payload.charCodeAt(0) - BYTE_OFFSET;
  const isDir = itemType === 0;

  const path = payload.slice(1);
  return isDir ? VFSNode.child(path) : VFSNode.file(path, itemType);
}
