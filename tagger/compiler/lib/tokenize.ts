export type Token =
  | { type: "IDENT"; value: string }
  | { type: "LBRACE" }
  | { type: "RBRACE" }
  | { type: "STAR" }
  | { type: "PLUS" }
  | { type: "HASH" };

export function tokenize(input: string): Token[] {
  const tokens: Token[] = [];
  let i = 0;

  while (i < input.length) {
    const c = input[i];

    // COMMENT //
    if (c === "/" && input[i + 1] === "/") {
      i += 2;
      while (i < input.length && input[i] !== "\n") i++;
      continue;
    }

    if (/\s/.test(c)) {
      i++;
      continue;
    }

    if (c === "{") {
      tokens.push({ type: "LBRACE" });
      i++;
      continue;
    }

    if (c === "}") {
      tokens.push({ type: "RBRACE" });
      i++;
      continue;
    }

    if (c === "*") {
      tokens.push({ type: "STAR" });
      i++;
      continue;
    }

    if (c === "+") {
      tokens.push({ type: "PLUS" });
      i++;
      continue;
    }

    if (c === "#") {
      tokens.push({ type: "HASH" });
      i++;
      continue;
    }

    // IDENT
    const start = i;
    while (i < input.length && /[a-zA-Z0-9_]/.test(input[i])) {
      i++;
    }

    if (start !== i) {
      tokens.push({
        type: "IDENT",
        value: input.slice(start, i),
      });
      continue;
    }

    throw new Error(`Unexpected char: ${c}`);
  }

  return tokens;
}
