import type { AST } from "./ast";

export function compile(ast: AST[]) {
  const result: Record<string, Record<string, string[]>> = {};

  function walk(nodes: AST[], group?: string) {
    for (const n of nodes) {
      if (n.type === "group") {
        walk(n.body, n.name);
      } else {
        const key = n.key;

        if (!result[group || key]) {
          result[group || key] = {};
        }

        result[group || key][key] = n.outputs;
      }
    }
  }

  walk(ast);

  return result;
}
