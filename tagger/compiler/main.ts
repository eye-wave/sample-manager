import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { compileTree } from "./lib";

const path = join(import.meta.dirname, "..", "tags.tree");
const file = await readFile(path, "utf8");

console.log(file);

const out = compileTree(file);
console.log(out);
