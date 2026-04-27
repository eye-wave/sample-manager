import cssNano from "cssnano";
import { defineConfig } from "vite";
import injectHTML from "vite-plugin-html-inject";
import { viteSingleFile } from "vite-plugin-singlefile";
import htmlTemplateMinifyPlugin from "./src-build/minify-templates";

export default defineConfig({
  resolve: {
    alias: {
      "@assets": "/assets",
    },
  },
  css: {
    postcss: {
      plugins: [cssNano({ preset: "advanced" })],
    },
  },
  build: {
    modulePreload: false,
  },
  plugins: [htmlTemplateMinifyPlugin(), injectHTML(), viteSingleFile()],
});
