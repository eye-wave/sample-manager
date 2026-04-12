import { defineConfig } from "vite";
import htmlTemplateMinifyPlugin from "./src-build/minify-templates";

export default defineConfig({
  build: {
    modulePreload: false,
  },
  plugins: [htmlTemplateMinifyPlugin()],
});
