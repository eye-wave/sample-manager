import cssNano from "cssnano";
import htmlTemplateMinifyPlugin from "./src-build/minify-templates";
import injectHTML from "vite-plugin-html-inject";
import { betterEnums } from "vite-plugin-better-enums";
import { defineConfig } from "vite";
import { viteSingleFile } from "vite-plugin-singlefile";

const isProd = import.meta.env?.PROD ?? process.env.NODE_ENV === "production";

export default defineConfig({
  publicDir: isProd ? "public" : "../assets",
  css: {
    postcss: {
      plugins: [cssNano({ preset: "advanced" })],
    },
  },
  build: {
    modulePreload: false,
  },
  plugins: [htmlTemplateMinifyPlugin(), injectHTML(), viteSingleFile(), betterEnums()],
});
