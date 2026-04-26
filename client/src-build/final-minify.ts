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

  output = moveHeadScriptToBodyEnd(output);

  output = output.replace("\n/*$vite$:1*/", "").replace(" crossorigin type=module", "");

  await writeFile("dist/index.html", output);
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
