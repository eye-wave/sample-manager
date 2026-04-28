import { readFile, writeFile } from "node:fs/promises";
import { load } from "cheerio";
import { minify as minifyHTML } from "html-minifier-terser";
import { minify as minifyJS } from "terser";

main();
async function main() {
  const file = await readFile("dist/index.html", "utf8");

  let output = file;

  output = shortenHtmlIds(output);

  output = await minifyHTML(output, {
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

export const idFactory = (prefix: string) => {
  const alphabet = "abcdefghijklmnopqrstuvwxyz0123456789_ABCDEFGHIJKLMNOPQRSTUVWXYZ";

  let count = -1;

  return () => {
    let n = count++;

    let result = "";
    while (n >= 0) {
      result = alphabet[n % 63] + result;
      n = Math.floor(n / 63) - 1;
    }

    return prefix + result;
  };
};

function shortenHtmlIds(html: string): string {
  const $ = load(html);

  const gen = idFactory("x");
  const map = new Map<string, string>();

  $("[id]").each((_, el) => {
    const oldId = $(el).attr("id");
    if (oldId?.endsWith("__")) {
      if (!map.has(oldId)) {
        map.set(oldId, gen());
      }
    }
  });

  let output = html;

  for (const [oldId, newId] of map) {
    const escaped = oldId.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const re = new RegExp(escaped, "g");
    output = output.replace(re, newId);
  }

  return output;
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
