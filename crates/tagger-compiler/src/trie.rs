use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::parser::{Item, Output, WordDecl};

// NFA Trie  (compiler-internal, heap-allocated)

// Each node can have *multiple* children for the same char because repeating
// nodes create genuine forks (e.g. "re+se" and "res" share 'r' but then need
// two distinct 'e' children).  We resolve this NFA to a DFA via subset
// construction before emitting the binary.

#[derive(Debug, Default)]
struct NfaNode {
    /// char -> set of target NFA node indices
    children: HashMap<char, Vec<usize>>,
    /// indices into the global string table
    output_ids: Vec<usize>,
    strict: bool,
}

struct NfaTrie {
    nodes: Vec<NfaNode>,
}

impl NfaTrie {
    fn new() -> Self {
        Self {
            nodes: vec![NfaNode::default()],
        }
    }

    fn insert(&mut self, decl: &WordDecl, inherited: &[String], st: &mut StringTable) {
        let word_str: String = decl
            .pattern
            .iter()
            .filter(|pc| !pc.optional)
            .map(|pc| pc.ch)
            .collect();

        if decl.outputs.is_empty() && inherited.is_empty() {
            eprintln!("warning: '{word_str}' has no outputs and no enclosing group - skipped");
            return;
        }

        let mut frontier: Vec<usize> = vec![0];

        for pc in &decl.pattern {
            let ch = pc.ch;

            if pc.optional {
                let new_idx = self.nodes.len();
                self.nodes.push(NfaNode::default());

                for &cur in &frontier {
                    self.nodes[cur]
                        .children
                        .entry(ch)
                        .or_default()
                        .push(new_idx);
                }

                let mut next = vec![new_idx];
                next.extend_from_slice(&frontier);
                frontier = next;
            } else {
                // For a plain step: allocate a fresh node.
                // For a repeating step: also allocate fresh, then add self-loop.
                // We intentionally do NOT reuse existing children here — that
                // was the source of the NFA-vs-DFA confusion.  Subset
                // construction will merge states correctly.
                let new_idx = self.nodes.len();
                self.nodes.push(NfaNode::default());

                for &cur in &frontier {
                    self.nodes[cur]
                        .children
                        .entry(ch)
                        .or_default()
                        .push(new_idx);
                }

                if pc.repeating {
                    self.nodes[new_idx]
                        .children
                        .entry(ch)
                        .or_default()
                        .push(new_idx);
                }

                frontier = vec![new_idx];
            }
        }

        for &term in &frontier {
            let node = &mut self.nodes[term];
            if decl.strict {
                node.strict = true;
            }

            let mut push = |s: String| {
                let id = st.intern(s);
                if !node.output_ids.contains(&id) {
                    node.output_ids.push(id);
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

    /// Follow all NFA edges for a set of states on char `ch`.
    fn follow(&self, states: &BTreeSet<usize>, ch: char) -> BTreeSet<usize> {
        let mut result = BTreeSet::new();
        for &s in states {
            if let Some(targets) = self.nfa_node(s).children.get(&ch) {
                result.extend(targets.iter().copied());
            }
        }
        result
    }

    fn nfa_node(&self, idx: usize) -> &NfaNode {
        &self.nodes[idx]
    }
}

// DFA Trie  (result of subset construction)

#[derive(Debug, Default)]
pub struct DfaNode {
    /// char -> DFA node index (deterministic, at most one per char)
    pub children: BTreeMap<char, usize>,
    pub output_ids: Vec<usize>,
    pub strict: bool,
}

pub struct DfaTrie {
    pub nodes: Vec<DfaNode>,
}

/// Build the NFA trie from source items, then convert to DFA via subset construction.
pub fn build_dfa(items: &[Item], st: &mut StringTable) -> DfaTrie {
    let mut nfa = NfaTrie::new();
    insert_items(items, &mut nfa, st, &[]);
    nfa_to_dfa(&nfa)
}

fn insert_items(items: &[Item], nfa: &mut NfaTrie, st: &mut StringTable, chain: &[String]) {
    for item in items {
        match item {
            Item::Word(decl) => nfa.insert(decl, chain, st),
            Item::Group { name, items: body } => {
                let mut next = chain.to_vec();
                next.push(name.clone());
                insert_items(body, nfa, st, &next);
            }
        }
    }
}

fn nfa_to_dfa(nfa: &NfaTrie) -> DfaTrie {
    // Each DFA state is a *set* of NFA states.
    let start: BTreeSet<usize> = std::iter::once(0).collect();

    // map: NFA state-set -> DFA node index
    let mut state_map: HashMap<BTreeSet<usize>, usize> = HashMap::new();
    let mut worklist: Vec<BTreeSet<usize>> = Vec::new();
    let mut dfa = DfaTrie { nodes: Vec::new() };

    let start_idx = 0usize;
    state_map.insert(start.clone(), start_idx);
    dfa.nodes.push(DfaNode::default());
    worklist.push(start);

    while let Some(state_set) = worklist.pop() {
        let dfa_idx = state_map[&state_set];

        // collect all output_ids and strict flag from every NFA node in the set
        let mut combined_outputs: Vec<usize> = Vec::new();
        let mut combined_strict = false;
        for &nfa_idx in &state_set {
            let nn = nfa.nfa_node(nfa_idx);
            combined_strict |= nn.strict;
            for &oid in &nn.output_ids {
                if !combined_outputs.contains(&oid) {
                    combined_outputs.push(oid);
                }
            }
        }
        dfa.nodes[dfa_idx].output_ids = combined_outputs;
        dfa.nodes[dfa_idx].strict = combined_strict;

        // collect all outgoing chars
        let mut chars: BTreeSet<char> = BTreeSet::new();
        for &nfa_idx in &state_set {
            chars.extend(nfa.nfa_node(nfa_idx).children.keys().copied());
        }

        for ch in chars {
            let next_set = nfa.follow(&state_set, ch);
            if next_set.is_empty() {
                continue;
            }

            let next_dfa_idx = if let Some(&idx) = state_map.get(&next_set) {
                idx
            } else {
                let idx = dfa.nodes.len();
                dfa.nodes.push(DfaNode::default());
                state_map.insert(next_set.clone(), idx);
                worklist.push(next_set);
                idx
            };

            dfa.nodes[dfa_idx].children.insert(ch, next_dfa_idx);
        }
    }

    dfa
}

// String table

pub struct StringTable {
    map: HashMap<String, usize>,
    entries: Vec<String>,
}

impl StringTable {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            entries: Vec::new(),
        }
    }

    pub fn intern(&mut self, s: String) -> usize {
        if let Some(&id) = self.map.get(&s) {
            return id;
        }
        let id = self.entries.len();
        self.map.insert(s.clone(), id);
        self.entries.push(s);
        id
    }

    pub fn build_dict(&self) -> (Vec<u8>, Vec<(u16, u8)>) {
        let mut blob = Vec::new();
        let mut ranges = Vec::new();
        for s in &self.entries {
            let start = blob.len();
            let len = s.len();
            assert!(start <= 0xFFFF, "dict exceeds u16 address space");
            assert!(len <= 0xFF, "tag longer than 255 bytes: {s:?}");
            blob.extend_from_slice(s.as_bytes());
            ranges.push((start as u16, len as u8));
        }
        (blob, ranges)
    }
}

// Binary emitter

const MAX_U16: usize = 0xFFFF;

fn char_to_key(ch: char) -> u8 {
    tagger_charset::encode(ch).unwrap_or_else(|| panic!("unsupported char in trie: {ch:?}"))
}

impl DfaTrie {
    /// Log connection offset distribution to help decide whether a delta
    /// encoding scheme is worthwhile.
    pub fn log_offset_stats(&self) {
        // Pass 1: compute byte offsets (same logic as emit_binary pass 1)
        let mut offsets = Vec::with_capacity(self.nodes.len());
        let mut cursor = 0usize;
        for node in &self.nodes {
            offsets.push(cursor);
            cursor += 2 + node.children.len() * 3 + node.output_ids.len() * 3;
        }

        let mut total = 0usize;
        let mut fits_i8 = 0usize; // |delta| <= 127
        let mut fits_u8 = 0usize; // delta >= 0 && delta <= 255
        let mut fits_i16 = 0usize; // |delta| <= 32767
        let mut max_delta: i64 = 0;
        let mut min_delta: i64 = 0;

        for (node_idx, node) in self.nodes.iter().enumerate() {
            let src_off = offsets[node_idx] as i64;
            for &target_idx in node.children.values() {
                let dst_off = offsets[target_idx] as i64;
                let delta = dst_off - src_off;
                total += 1;
                if delta >= -128 && delta <= 127 {
                    fits_i8 += 1;
                }
                if delta >= 0 && delta <= 255 {
                    fits_u8 += 1;
                }
                if delta >= -32768 && delta <= 32767 {
                    fits_i16 += 1;
                }
                if delta > max_delta {
                    max_delta = delta;
                }
                if delta < min_delta {
                    min_delta = delta;
                }
            }
        }

        eprintln!("=== connection offset stats ===");
        eprintln!("  total connections : {total}");
        if total > 0 {
            eprintln!("  delta range       : {min_delta} .. {max_delta}");
            eprintln!(
                "  fits i8  (-128..127)   : {fits_i8} / {total}  ({:.1}%)",
                fits_i8 as f64 / total as f64 * 100.0
            );
            eprintln!(
                "  fits u8  (0..255)      : {fits_u8} / {total}  ({:.1}%)",
                fits_u8 as f64 / total as f64 * 100.0
            );
            eprintln!(
                "  fits i16 (-32768..32767): {fits_i16} / {total}  ({:.1}%)",
                fits_i16 as f64 / total as f64 * 100.0
            );
        }

        // Per-node delta analysis
        let mut nodes_all_i8 = 0usize;
        let mut nodes_any_conn = 0usize;
        let mut nodes_mixed = 0usize;

        for (node_idx, node) in self.nodes.iter().enumerate() {
            if node.children.is_empty() {
                continue;
            }
            nodes_any_conn += 1;
            let src_off = offsets[node_idx] as i64;
            let all_fit = node.children.values().all(|&t| {
                let d = offsets[t] as i64 - src_off;
                d >= -128 && d <= 127
            });
            if all_fit {
                nodes_all_i8 += 1;
            } else {
                nodes_mixed += 1;
            }
        }

        eprintln!("  --- per-node delta coverage ---");
        eprintln!("  nodes with connections    : {nodes_any_conn}");
        if nodes_any_conn > 0 {
            eprintln!(
                "  all connections fit i8    : {nodes_all_i8} / {nodes_any_conn}  ({:.1}%)",
                nodes_all_i8 as f64 / nodes_any_conn as f64 * 100.0
            );
            eprintln!(
                "  at least one needs u16    : {nodes_mixed} / {nodes_any_conn}  ({:.1}%)",
                nodes_mixed as f64 / nodes_any_conn as f64 * 100.0
            );
        }
        eprintln!("================================");
    }
}

/// Statistics about a compiled DFA trie, printed via `log_stats`.
pub struct TrieStats {
    pub node_count: usize,
    pub max_connections: u8,
    pub avg_connections: f64,
    pub max_outputs: u8,
    pub avg_outputs: f64,
}

impl DfaTrie {
    pub fn stats(&self) -> TrieStats {
        let mut total_conn = 0usize;
        let mut max_conn = 0u8;
        let mut total_out = 0usize;
        let mut max_out = 0u8;

        for node in &self.nodes {
            let nc = node.children.len() as u8;
            let no = node.output_ids.len() as u8;
            total_conn += nc as usize;
            total_out += no as usize;
            if nc > max_conn {
                max_conn = nc;
            }
            if no > max_out {
                max_out = no;
            }
        }

        let n = self.nodes.len();
        TrieStats {
            node_count: n,
            max_connections: max_conn,
            avg_connections: if n > 0 {
                total_conn as f64 / n as f64
            } else {
                0.0
            },
            max_outputs: max_out,
            avg_outputs: if n > 0 {
                total_out as f64 / n as f64
            } else {
                0.0
            },
        }
    }

    pub fn log_stats(&self) {
        let s = self.stats();
        eprintln!("=== trie stats ===");
        eprintln!("  nodes:       {}", s.node_count);
        eprintln!(
            "  connections  max={} avg={:.2}",
            s.max_connections, s.avg_connections
        );
        eprintln!(
            "  outputs      max={} avg={:.2}",
            s.max_outputs, s.avg_outputs
        );
        eprintln!("==================");
    }
}

pub fn emit_binary(dfa: &DfaTrie, st: &StringTable) -> Vec<u8> {
    let (dict_blob, str_ranges) = st.build_dict();

    // Fixpoint: node connection strides depend on whether deltas fit i8,
    // which depends on target offsets, which depend on strides.
    // Start assuming all nodes use delta (2 bytes/conn), iterate until stable.
    let n = dfa.nodes.len();
    let mut use_delta: Vec<bool> = vec![true; n];

    let compute_offsets = |use_delta: &Vec<bool>| -> Vec<usize> {
        let mut offsets = Vec::with_capacity(n);
        let mut cursor = 0usize;
        for (i, node) in dfa.nodes.iter().enumerate() {
            offsets.push(cursor);
            let conn_bytes = if use_delta[i] { 2 } else { 3 };
            cursor += 2 + node.children.len() * conn_bytes + node.output_ids.len() * 3;
        }
        offsets
    };

    // Iterate until the delta assignment stabilises
    loop {
        let offsets = compute_offsets(&use_delta);
        let mut changed = false;
        for (i, node) in dfa.nodes.iter().enumerate() {
            if node.children.is_empty() {
                continue;
            }
            let src_off = offsets[i] as i64;
            let all_fit = node.children.values().all(|&t| {
                let d = offsets[t] as i64 - src_off;
                d >= -128 && d <= 127
            });
            if use_delta[i] != all_fit {
                use_delta[i] = all_fit;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    let node_offsets = compute_offsets(&use_delta);

    // Serialise
    let mut tree: Vec<u8> = Vec::new();
    for (i, node) in dfa.nodes.iter().enumerate() {
        let is_delta = use_delta[i];

        // header: bit15=strict | bits14..10=num_outputs | bit9=delta_flag | bits8..0=num_connections
        assert!(node.children.len() <= 511, "node has >511 children");
        assert!(node.output_ids.len() <= 31, "node has >31 outputs");
        let header: u16 = ((node.strict as u16) << 15)
            | ((node.output_ids.len() as u16) << 10)
            | ((is_delta as u16) << 9)
            | (node.children.len() as u16);
        tree.extend_from_slice(&header.to_le_bytes());

        // connections
        let src_off = node_offsets[i] as i64;
        for (&ch, &target_node_idx) in &node.children {
            let key = char_to_key(ch);
            let dst_off = node_offsets[target_node_idx];
            if is_delta {
                let delta = dst_off as i64 - src_off;
                debug_assert!(delta >= -128 && delta <= 127);
                tree.push(key);
                tree.push(delta as i8 as u8);
            } else {
                assert!(
                    dst_off <= MAX_U16,
                    "tree chunk exceeds u16 address space ({dst_off})"
                );
                tree.push(key);
                tree.extend_from_slice(&(dst_off as u16).to_le_bytes());
            }
        }

        // outputs
        for &sid in &node.output_ids {
            let (start, len) = str_ranges[sid];
            tree.extend_from_slice(&start.to_le_bytes());
            tree.push(len);
        }
    }

    let mut out = Vec::new();
    write_chunk(&mut out, &dict_blob);
    write_chunk(&mut out, &tree);
    out
}

fn write_chunk(buf: &mut Vec<u8>, data: &[u8]) {
    buf.extend_from_slice(&(data.len() as u32).to_le_bytes());
    buf.extend_from_slice(data);
}
