export default function htmlTemplateMinifyPlugin() {
  return {
    name: "vite:html-template-tools",
    enforce: "pre",

    async transform(code: string, id: string) {
      if (!id.match(/\.(ts|js|tsx|jsx)$/)) return;

      let result = code;

      // strip DEV blocks
      if (process.env.NODE_ENV === "production") {
        result = result.replace(/\/\/\/\s*DEV start[\s\S]*?\/\/\/\s*DEV end/g, "");
      }

      // minify /* HTML */ templates
      const regex = /\/\*\s*HTML\s*\*\/\s*`([\s\S]*?)`/g;

      let match: RegExpMatchArray | null;
      // biome-ignore lint/suspicious/noAssignInExpressions: trust
      while ((match = regex.exec(result))) {
        const fullMatch = match[0];
        const templateContent = match[1];

        if (!templateContent) continue;

        const expressions: string[] = [];

        const html = templateContent.replace(/\$\{([\s\S]*?)\}/g, (_, expr) => {
          const i = expressions.length;
          expressions.push(expr);
          return `___EXPR_${i}___`;
        });

        const minified = await import("html-minifier-terser").then((m) =>
          m.minify(html, {
            collapseWhitespace: true,
            removeComments: true,
          }),
        );

        let final = minified;

        expressions.forEach((expr, i) => {
          final = final.replace(`___EXPR_${i}___`, `\${${expr}}`);
        });

        result = result.replace(fullMatch, "`" + final + "`");
      }

      return result;
    },
  };
}
