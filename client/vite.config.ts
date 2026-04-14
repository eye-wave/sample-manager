import { defineConfig } from "vite";
import injectHTML from "vite-plugin-html-inject";
import { viteSingleFile } from "vite-plugin-singlefile";
import htmlTemplateMinifyPlugin from "./src-build/minify-templates";

export default defineConfig({
  build: {
    modulePreload: false,
  },
  plugins: [htmlTemplateMinifyPlugin(), injectHTML(), viteSingleFile()],
});
