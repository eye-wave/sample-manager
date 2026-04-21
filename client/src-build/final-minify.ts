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
  output = moveHeadScriptToBodyEnd(output);

  output = output.replace("\n/*$vite$:1*/", "").replace(" crossorigin type=module", "");

  await writeFile("dist/index.html", output);
}

function renameCssVariables(html: string) {
  const varMap = new Map<string, string>();
  let index = 0;

  const getName = (i: number) => `--${String.fromCharCode(65 + i)}`;

  const usedVars = new Set<string>();
  html.replace(/var\(\s*(--[a-zA-Z0-9-_]+)/g, (_, v) => {
    usedVars.add(v);
    return "";
  });

  html = html.replace(/(--[a-zA-Z0-9-_]+)\s*:\s*[^;}{]+;/g, (full, name) => {
    if (!usedVars.has(name)) return "";
    return full;
  });

  html = html.replace(/--[a-zA-Z0-9-_]+/g, (match) => {
    if (!usedVars.has(match)) return match;

    if (!varMap.has(match)) {
      varMap.set(match, getName(index++));
    }
    return varMap.get(match) ?? match;
  });

  return html;
}

function moveHeadScriptToBodyEnd(html: string) {
  const scriptMatch = html.match(
    /<head[^>]*>[\s\S]*?<script[\s\S]*?<\/script>[\s\S]*?<\/head>/i,
  );
  if (!scriptMatch) return html;

  const headBlock = scriptMatch[0];

  const scriptTagMatch = headBlock.match(/<script[\s\S]*?<\/script>/i);
  if (!scriptTagMatch) return html;

  const scriptTag = scriptTagMatch[0];

  const withoutScript = html.replace(scriptTag, "");

  return withoutScript.replace(/<\/body>/i, `${scriptTag}</body>`);
}
