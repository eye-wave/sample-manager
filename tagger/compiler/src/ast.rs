use std::collections::HashMap;

use crate::parser::{Item, Output, WordDecl};

#[derive(Debug, Default)]
struct TrieNode {
    children: HashMap<char, usize>,
    outputs: Vec<String>,
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
        let word_str: String = decl.pattern.iter().map(|pc| pc.ch).collect();
        let mut cur = 0usize;

        for pc in &decl.pattern {
            let ch = pc.ch;

            let child = match self.nodes[cur].children.get(&ch).copied() {
                Some(idx) => idx,
                None => {
                    let idx = self.nodes.len();
                    self.nodes.push(TrieNode::default());
                    self.nodes[cur].children.insert(ch, idx);
                    idx
                }
            };

            if pc.repeating {
                self.nodes[child].children.insert(ch, child);
            }

            cur = child;
        }

        let node = &mut self.nodes[cur];

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
        if decl.outputs.is_empty() && inherited.is_empty() {
            eprintln!("warning: '{word_str}' has no outputs and no enclosing group — skipped");
            return;
        }
        for g in inherited {
            push(g.clone());
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
    out.push_str("#include \"../node.h\"\n\n");
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
                    format!("[{offset}] = &nodes[{idx}]")
                })
                .collect();
            format!(".next = {{{}}}", entries.join(", "))
        });

        let output_part = has_output.then(|| {
            let arr: Vec<String> = node.outputs.iter().map(|s| format!("\"{s}\"")).collect();
            let len = node.outputs.len();
            // .output is const char**; compound literal keeps the address valid
            // at file scope in C99+.
            format!(
                ".output = (const char*[{len}]){{{}}}, .len = {len}",
                arr.join(", ")
            )
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
