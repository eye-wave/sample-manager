import { readFile, writeFile } from "node:fs/promises";
import { minify } from "html-minifier-terser";

main();
async function main() {
  const file = await readFile("dist/index.html", "utf8");
  let output = await minify(file, {
    collapseWhitespace: true,
    collapseInlineTagWhitespace: true,
    removeComments: true,
    sortAttributes: true,
    sortClassName: true,
    removeAttributeQuotes: true,
    collapseBooleanAttributes: true,
  });

  output = renameCssVariables(output);

  await writeFile("dist/index.html", output);
}

function renameCssVariables(html: string) {
  const varMap = new Map();
  let index = 0;

  const getName = (i: number) => `--${String.fromCharCode(65 + i)}`;

  html = html.replace(/--[a-zA-Z0-9-_]+/g, (match) => {
    if (!varMap.has(match)) {
      varMap.set(match, getName(index++));
    }
    return varMap.get(match);
  });

  return html;
}
