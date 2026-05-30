#!/usr/bin/env bun

import { readFile, writeFile } from "node:fs/promises";
import { load } from "cheerio";

type Grouped = Map<string, { name: string; tsType: string }[]>;

const html: string = await readFile("index.html", "utf-8");
const $ = load(html);

const groups: Grouped = new Map();

$("[id]").each((_, _el) => {
  const el = $(_el);

  const id: string | undefined = el.attr("id");
  const tag: string | undefined = el.get(0)?.tagName;

  if (!id || !tag) return;
  if (!isValidId(id)) return;

  const tsType: string = toHtmlElement(tag);

  if (!groups.has(tag)) {
    groups.set(tag, []);
  }

  groups.get(tag)?.push({ name: id, tsType });
});

const sortedTags = [...groups.keys()].sort();

let out: string = `// AUTO-GENERATED FILE - DO NOT EDIT\n\n`;

for (const tag of sortedTags) {
  const items = groups.get(tag);
  if (!items) continue;

  items.sort((a, b) => a.name.localeCompare(b.name));

  out += `// ${tag.toUpperCase()}\n`;

  for (const { name, tsType } of items) {
    out += `declare const ${name}: ${tsType};\n`;
  }

  out += `\n`;
}

await writeFile("src/gen/elements.d.ts", out);

function toHtmlElement(tag: string): string {
  return `HTML${tag[0].toUpperCase()}${tag.slice(1)}Element`;
}

function isValidId(id: string) {
  if (!id.endsWith("__")) return false;

  const base = id.slice(0, -2);

  return /^[a-zA-Z_$][a-zA-Z0-9_$]*$/.test(base);
}
