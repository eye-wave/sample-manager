use std::collections::HashMap;

use crate::parser::{Item, Output, WordDecl};

#[derive(Debug, Default)]
struct TrieNode {
    children: HashMap<char, usize>,
    outputs: Vec<String>,
    strict: bool,
}

pub struct Trie {
    nodes: Vec<TrieNode>,
}

impl Trie {
    pub fn new() -> Self {
        Self {
            nodes: vec![TrieNode::default()],
        }
    }

    fn insert(&mut self, decl: &WordDecl, inherited: &[String]) {
        let word_str: String = decl
            .pattern
            .iter()
            .filter(|pc| !pc.optional)
            .map(|pc| pc.ch)
            .collect();

        if decl.outputs.is_empty() && inherited.is_empty() {
            eprintln!("warning: '{word_str}' has no outputs and no enclosing group — skipped");
            return;
        }

        let mut frontier: Vec<usize> = vec![0];

        for pc in &decl.pattern {
            let ch = pc.ch;

            if pc.optional {
                let mut next_frontier: Vec<usize> = Vec::new();

                let shared = self.nodes.len();
                self.nodes.push(TrieNode::default());

                for &cur in &frontier {
                    self.nodes[cur].children.insert(ch, shared);
                }

                next_frontier.push(shared);
                next_frontier.extend_from_slice(&frontier);

                frontier = next_frontier;
            } else {
                let shared = frontier
                    .iter()
                    .find_map(|&cur| self.nodes[cur].children.get(&ch).copied())
                    .unwrap_or_else(|| {
                        let idx = self.nodes.len();
                        self.nodes.push(TrieNode::default());
                        idx
                    });

                for &cur in &frontier {
                    self.nodes[cur].children.insert(ch, shared);
                }

                if pc.repeating {
                    self.nodes[shared].children.insert(ch, shared);
                }

                frontier = vec![shared];
            }
        }

        for &term in &frontier {
            let node = &mut self.nodes[term];
            if decl.strict {
                node.strict = true;
            }
            let mut push = |s: String| {
                if !node.outputs.contains(&s) {
                    node.outputs.push(s);
                }
            };
            for out in &decl.outputs {
                push(match out {
                    Output::Itself => word_str.clone(),
                    Output::Named(name) => name.clone(),
                });
            }
            for g in inherited {
                push(g.clone());
            }
        }
    }
}

pub fn build_trie(items: &[Item], trie: &mut Trie, chain: &[String]) {
    for item in items {
        match item {
            Item::Word(decl) => trie.insert(decl, chain),
            Item::Group { name, items: body } => {
                let mut next = chain.to_vec();
                next.push(name.clone());
                build_trie(body, trie, &next);
            }
        }
    }
}

pub fn emit_c(trie: &Trie) -> String {
    let mut out = String::new();
    out.push_str("#pragma once\n\n");
    out.push_str("#include <stdint.h>\n");
    out.push_str("#include \"../node.h\"\n\n");

    let mut string_index: Vec<String> = Vec::new();
    let mut string_map: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for node in &trie.nodes {
        for s in &node.outputs {
            if !string_map.contains_key(s) {
                string_map.insert(s.clone(), string_index.len());
                string_index.push(s.clone());
            }
        }
    }

    if !string_index.is_empty() {
        out.push_str("static const char *output_strings[] = {\n");
        for s in &string_index {
            out.push_str(&format!("    \"{s}\",\n"));
        }
        out.push_str("};\n\n");
    }

    out.push_str("static Node nodes[] = {\n");

    for node in &trie.nodes {
        let mut children: Vec<(char, usize)> =
            node.children.iter().map(|(&c, &n)| (c, n)).collect();
        children.sort_by_key(|&(c, _)| c);

        let has_children = !children.is_empty();
        let has_output = !node.outputs.is_empty();

        if !has_children && !has_output {
            out.push_str("    {},\n");
            continue;
        }

        let next_part = has_children.then(|| {
            let entries: Vec<String> = children
                .iter()
                .map(|(ch, idx)| {
                    let offset = match *ch {
                        '_' => 26,
                        'a'..='z' => (*ch as u8 - b'a') as usize,
                        '0'..='9' => (*ch as u8 - b'0' + 27) as usize,
                        _ => panic!("unsupported char in trie: {ch:?}"),
                    };
                    format!("[{offset}] = {idx}")
                })
                .collect();
            format!(".next = {{{}}}", entries.join(", "))
        });

        let output_part = has_output.then(|| {
            let ptrs: Vec<String> = node
                .outputs
                .iter()
                .map(|s| {
                    let i = string_map[s];
                    format!("{i}")
                })
                .collect();
            let mut s = format!(".output = {{{}}}", ptrs.join(", "));

            if node.strict {
                s.push_str(", .strict = 1");
            }
            s
        });

        let parts: Vec<String> = [next_part, output_part].into_iter().flatten().collect();

        match parts.as_slice() {
            [single] => out.push_str(&format!("    {{{single}}},\n")),
            _ => {
                out.push_str("    {\n");
                for p in &parts {
                    out.push_str(&format!("        {p},\n"));
                }
                out.push_str("    },\n");
            }
        }
    }

    out.push_str("};\n");
    out
}
