import { parse } from "./lib/ast";
import { compile } from "./lib/compile";
import { tokenize } from "./lib/tokenize";

export function compileTree(text: string) {
  const tokens = tokenize(text);
  const ast = parse(tokens);

  return compile(ast);
}
