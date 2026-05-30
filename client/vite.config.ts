import cssNano from "cssnano";
import { defineConfig } from "vite";
import injectHTML from "vite-plugin-html-inject";
import { viteSingleFile } from "vite-plugin-singlefile";
import htmlTemplateMinifyPlugin from "./src-build/minify-templates";
import { betterEnums } from "vite-plugin-better-enums";

export default defineConfig({
  publicDir: "../assets",
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
