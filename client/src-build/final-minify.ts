#!/usr/bin/env bun

import { readFile, writeFile } from "node:fs/promises";
import { load } from "cheerio";
import { minify as minifyHTML } from "html-minifier-terser";
import { minify as minifyJS } from "terser";

main();
async function main() {
  const file = await readFile("dist/index.html", "utf8");

  let output = file;

  output = moveHeadScriptToBodyEnd(output);
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

  output = commonDictionary(output);
  output = output
    .replace("\n/*$vite$:1*/", "")
    .replaceAll('crossorigin=""', "")
    .replaceAll("type=module", "");

  output = await minifyInlineScripts(output);

  await writeFile("dist/index.html", output);
}

function moveHeadScriptToBodyEnd(html: string) {
  const $ = load(html);

  const script = $("script").first();
  script.remove();

  $("body").append(script);

  return $.html();
}

export const idFactory = (
  prefix: string,
  alphabet = "abcdefghijklmnopqrstuvwxyz0123456789_ABCDEFGHIJKLMNOPQRSTUVWXYZ",
) => {
  let count = 0;

  return (i?: number) => {
    let n = i ?? count++;

    let result = "";
    while (n >= 0) {
      result = alphabet[n % alphabet.length] + result;
      n = Math.floor(n / alphabet.length) - 1;
    }

    return prefix + result;
  };
};

function shortenHtmlIds(html: string): string {
  const $ = load(html);

  const gen = idFactory("X", "abcdefghijklmnopqrstuvwxyz0123456789_");
  const map = new Map<string, string>();

  $("[id]").each((_, el) => {
    const oldId = $(el).attr("id");
    if (!oldId?.endsWith("__")) return;
    if (map.has(oldId)) return;

    map.set(oldId, gen());
  });

  let output = html;

  for (const [oldId, newId] of map) {
    let ref = 0;

    output = output.replaceAll(oldId, () => {
      ref += 1;
      return newId;
    });

    if (ref === 1) {
      console.warn(`Element ${oldId} is unused.`);
      output = output.replaceAll(`id="${newId}"`, "");
    }
  }

  return output;
}

function commonDictionary(html: string) {
  const keywords = [
    "addEventListener",
    "removeEventListener",
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
    "clientX",
    "clientY",
    "style",
    "dataset",
    "parentElement",
    "preventDefault",
    "querySelectorAll",
    "querySelector",
    "setAttribute",
    "textContent",
  ];

  const gen = idFactory("__DICT__");

  let dictCode = "const ";

  keywords.forEach((k, i) => {
    const id = gen(i);

    const r1 = new RegExp(`\\?\\.${k}`, "g");
    const r2 = new RegExp(`\\.${k}`, "g");
    const r3 = new RegExp(`\`${k}\``, "g");

    html = html.replace(r1, `?.[${id}]`);
    html = html.replace(r2, `[${id}]`);
    html = html.replace(r3, id);

    dictCode += `${id}="${k}",`;
  });

  dictCode = dictCode.replace(/,$/, ";");

  html = html.replace(/<script[^>]*>/, "<script>" + dictCode);

  return html;
}

export async function minifyInlineScripts(html: string): Promise<string> {
  const $ = load(html);

  const scripts = $("script");

  for (let i = 0; i < scripts.length; i++) {
    const el = scripts[i];
    const script = $(el);

    const code = script.html();
    if (!code?.trim()) continue;

    const result = await minifyJS(code, {
      compress: true,
      mangle: {
        toplevel: true,
      },
    });

    if (!result.code) continue;

    script.html(result.code);
  }

  return $.html();
}
