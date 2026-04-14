import { readFile, writeFile } from "node:fs/promises";
import { minify } from "html-minifier-terser";

const file = await readFile("dist/index.html", "utf8");
const output = await minify(file, {
  collapseWhitespace: true,
  collapseInlineTagWhitespace: true,
  removeComments: true,
  sortAttributes: true,
  sortClassName: true,
  removeAttributeQuotes: true,
  collapseBooleanAttributes: true,
});

await writeFile("dist/index.html", output);
