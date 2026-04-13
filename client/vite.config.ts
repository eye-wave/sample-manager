import { defineConfig } from "vite";
import { ViteMinifyPlugin } from "vite-plugin-minify";
import htmlTemplateMinifyPlugin from "./src-build/minify-templates";

export default defineConfig({
  build: {
    modulePreload: false,
  },
  plugins: [htmlTemplateMinifyPlugin(), ViteMinifyPlugin()],
});
