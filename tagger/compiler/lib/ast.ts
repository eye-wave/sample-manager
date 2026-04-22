import type { Token } from "./tokenize";

export type AST = GroupNode | MappingNode;

interface GroupNode {
  type: "group";
  name: string;
  body: AST[];
}

type Term = { type: "ident"; value: string; repeat?: boolean } | { type: "wildcard" };

interface MappingNode {
  type: "mapping";
  key: string;
  pattern: Term[];
  outputs: string[];
}

export function parse(tokens: Token[]) {
  let i = 0;

  function peek() {
    return tokens[i];
  }

  function eat() {
    return tokens[i++];
  }

  function expect(type: Token["type"]) {
    const t = eat();
    if (!t || t.type !== type) {
      throw new Error(`Expected ${type}`);
    }
    return t;
  }

  function parseFile(): AST[] {
    const nodes: AST[] = [];
    while (i < tokens.length) {
      nodes.push(parseStatement());
    }
    return nodes;
  }

  function parseStatement(): AST {
    const t = peek();
    if (t.type === "IDENT" && tokens[i + 1]?.type === "LBRACE") {
      return parseGroup();
    }
    return parseMapping();
  }

  function parseGroup(): GroupNode {
    const name = (eat() as any).value;
    expect("LBRACE");

    const body: AST[] = [];

    while (peek()?.type !== "RBRACE") {
      body.push(parseStatement());
    }

    expect("RBRACE");

    return {
      type: "group",
      name,
      body,
    };
  }

  function parseMapping(): MappingNode {
    const key = (expect("IDENT") as any).value;

    const pattern = parsePattern();

    expect("STAR");

    const outputs: string[] = [];

    outputs.push((expect("IDENT") as any).value);

    while (peek()?.type === "STAR") {
      eat();
      outputs.push((expect("IDENT") as any).value);
    }

    return {
      type: "mapping",
      key,
      pattern,
      outputs,
    };
  }

  function parsePattern(): Term[] {
    const terms: Term[] = [];

    while (true) {
      const t = peek();

      if (t.type === "IDENT") {
        const ident = eat().value;

        let repeat = false;

        if (peek()?.type === "PLUS") {
          eat();
          repeat = true;
        }

        terms.push({
          type: "ident",
          value: ident,
          repeat,
        });

        continue;
      }

      if (t.type === "HASH") {
        eat();
        terms.push({ type: "wildcard" });
        continue;
      }

      break;
    }

    return terms;
  }

  return parseFile();
}
