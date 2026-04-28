import { readFile, writeFile } from "node:fs/promises";
import { minify as minifyHTML } from "html-minifier-terser";
import { minify as minifyJS } from "terser";

main();
async function main() {
  const file = await readFile("dist/index.html", "utf8");

  let output = await minifyHTML(file, {
    collapseWhitespace: true,
    collapseInlineTagWhitespace: true,
    removeComments: true,
    sortAttributes: true,
    sortClassName: true,
    removeAttributeQuotes: true,
    collapseBooleanAttributes: true,
  });

  output = shortenHtmlIds(output);
  output = moveHeadScriptToBodyEnd(output);

  output = output.replace("\n/*$vite$:1*/", "").replace(" crossorigin type=module", "");
  output = commonDictionary(output);
  output = await minifyInlineScripts(output);

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

function shortenHtmlIds(html: string): string {
  const idRegex = /<[^>]+id=([^\s>]+)/g;

  const ids = new Set<string>(Array.from(html.matchAll(idRegex), (m) => m[1]));

  const generateId = (() => {
    let i = 0;
    return () => "x" + (++i).toString(36);
  })();

  const map = new Map<string, string>();

  for (const oldId of ids) {
    map.set(oldId, generateId());
  }

  for (const [oldId, newId] of map.entries()) {
    const escaped = oldId.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    html = html.replace(new RegExp(escaped, "g"), newId);
  }

  return html;
}

function commonDictionary(html: string) {
  const keywords = [
    "addEventListener",
    "appendChild",
    "beforeend",
    "classList",
    "className",
    "cloneNode",
    "innerHTML",
    "insertAdjacentHTML",
    "insertBefore",
    "keydown",
    "onclick",
    "preventDefault",
    "querySelector",
    "setAttribute",
    "textContent",
  ];

  const generateId = (i: number) => "d" + i.toString(36);

  let dictCode = "const ";

  keywords.forEach((k, i) => {
    const id = generateId(i);

    const r1 = new RegExp(`\\?\\.${k}`, "g");
    const r2 = new RegExp(`\\.${k}`, "g");
    const r3 = new RegExp(`\`${k}\``, "g");

    html = html.replace(r1, `?.[${id}]`);
    html = html.replace(r2, `[${id}]`);
    html = html.replace(r3, id);

    dictCode += `${id}="${k}",`;
  });

  dictCode = dictCode.replace(/,$/, ";");

  html = html.replace("<script>", "<script>" + dictCode);

  return html;
}

export async function minifyInlineScripts(html: string): Promise<string> {
  const scriptRegex = /<script\b[^>]*>([\s\S]*?)<\/script>/gi;

  const matches = [...html.matchAll(scriptRegex)];

  for (const match of matches) {
    const fullTag = match[0];
    const jsCode = match[1];

    if (!jsCode.trim()) continue;

    const result = await minifyJS(jsCode, {
      compress: true,
      mangle: {
        toplevel: true,
      },
    });

    if (!result.code) continue;

    html = html.replace(fullTag, fullTag.replace(jsCode, result.code));
  }

  return html;
}
