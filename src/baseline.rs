use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::sync::OnceLock;

use fancy_regex::Regex;

use crate::cache::BpeCache;
use crate::profile::{Profile, time};

const MERGES: &str = include_str!("../assets/vocab.bpe");
const VOCAB: &str = include_str!("../assets/encoder.json");
const PATTERN: &str = concat!(
    r"'s|'t|'re|'ve|'m|'ll|'d|",
    r" ?\p{L}+| ?\p{N}+|",
    r" ?[^\s\p{L}\p{N}]+|\s+(?!\S)|\s+",
);

static TOKENIZER: OnceLock<Tokenizer> = OnceLock::new();

pub fn encode(input: &str) -> Vec<u32> {
    let mut cache = BpeCache::default();
    tokenizer().encode(input, &mut cache)
}

pub fn encode_fast(input: &str) -> Vec<u32> {
    let mut cache = BpeCache::default();
    tokenizer().encode_fast(input, &mut cache)
}

pub fn profile(input: &str, fast: bool) -> Profile {
    let tokenizer = tokenizer();
    let mut cache = BpeCache::default();
    let mut profile = Profile::default();
    let matches = time(&mut profile.regex, || tokenizer.matches(input));
    let mut output = Vec::new();
    for piece in matches {
        let symbols = if fast {
            tokenizer.merge_fast_profile(piece, &mut cache, &mut profile)
        } else {
            tokenizer.merge_profile(piece, &mut cache, &mut profile)
        };
        output.extend(time(&mut profile.output, || tokenizer.token_ids(symbols)));
    }
    profile
}

pub struct Tokenizer {
    pattern: Regex,
    byte_ids: [u32; 256],
    merge_ranks: HashMap<(u32, u32), (usize, u32)>,
    symbols: Vec<String>,
    symbol_ids: HashMap<String, u32>,
    token_ids: HashMap<String, u32>,
    token_symbols: Vec<String>,
}

impl Tokenizer {
    fn encode(&self, input: &str, cache: &mut BpeCache) -> Vec<u32> {
        self.pattern
            .find_iter(input)
            // Regex matches are independent BPE inputs.
            .flat_map(|matched| self.merge(matched.unwrap().as_str(), cache))
            // Only finished BPE pieces have vocabulary IDs.
            .map(|symbol| self.token_ids[&self.symbols[symbol as usize]])
            .collect()
    }

    // Repeatedly merge the lowest-ranked adjacent pair.
    pub(crate) fn merge(&self, input: &str, cache: &mut BpeCache) -> Vec<u32> {
        let encoded = self.byte_encode(input);
        if let Some(merged) = cache.get(&encoded) {
            return self.parse_symbols(&merged);
        }
        let pieces = self.merge_encoded(&encoded);
        if encoded.chars().count() > 1 {
            cache.put(encoded, self.symbols_to_string(&pieces));
        }
        pieces
    }

    fn merge_profile(&self, input: &str, cache: &mut BpeCache, profile: &mut Profile) -> Vec<u32> {
        let encoded = time(&mut profile.bytes, || self.byte_encode(input));
        let cached = time(&mut profile.cache, || cache.get(&encoded));
        if let Some(merged) = cached {
            return self.parse_symbols(&merged);
        }
        let pieces = time(&mut profile.merge, || self.merge_encoded(&encoded));
        cache.put(encoded, self.symbols_to_string(&pieces));
        pieces
    }

    fn merge_encoded(&self, encoded: &str) -> Vec<u32> {
        let mut pieces = encoded
            .chars()
            .map(|symbol| self.symbol_ids[symbol.to_string().as_str()])
            .collect::<Vec<_>>();

        loop {
            let best = pieces
                .windows(2)
                // Pick the earliest merge in vocab.bpe.
                .filter_map(|pair| {
                    self.merge_ranks
                        .get(&(pair[0], pair[1]))
                        .map(|&(rank, result)| (pair[0], pair[1], rank, result))
                })
                .min_by_key(|pair| pair.2);

            let Some((first, second, _, result)) = best else {
                return pieces;
            };

            // GPT-2 applies this merge at every matching pair.
            pieces = merge_all(pieces, first, second, result);
        }
    }

    fn encode_fast(&self, input: &str, cache: &mut BpeCache) -> Vec<u32> {
        self.pattern
            .find_iter(input)
            .flat_map(|matched| self.merge_fast(matched.unwrap().as_str(), cache))
            .map(|symbol| self.token_ids[&self.symbols[symbol as usize]])
            .collect()
    }

    pub(crate) fn merge_fast(&self, input: &str, cache: &mut BpeCache) -> Vec<u32> {
        let encoded = self.byte_encode(input);
        if let Some(merged) = cache.get(&encoded) {
            return self.parse_symbols(&merged);
        }
        let pieces = self.merge_fast_encoded(&encoded);
        if encoded.chars().count() > 1 {
            cache.put(encoded, self.symbols_to_string(&pieces));
        }
        pieces
    }

    fn merge_fast_profile(
        &self,
        input: &str,
        cache: &mut BpeCache,
        profile: &mut Profile,
    ) -> Vec<u32> {
        let encoded = time(&mut profile.bytes, || self.byte_encode(input));
        let cached = time(&mut profile.cache, || cache.get(&encoded));
        if let Some(merged) = cached {
            return self.parse_symbols(&merged);
        }
        let pieces = time(&mut profile.merge, || self.merge_fast_encoded(&encoded));
        cache.put(encoded, self.symbols_to_string(&pieces));
        pieces
    }

    fn merge_fast_encoded(&self, encoded: &str) -> Vec<u32> {
        let mut nodes = encoded
            .chars()
            .enumerate()
            .map(|(index, symbol)| Node {
                symbol: self.symbol_ids[symbol.to_string().as_str()],
                previous: index.checked_sub(1),
                next: (index + 1 < encoded.chars().count()).then_some(index + 1),
                live: true,
            })
            .collect::<Vec<_>>();
        // Lowest rank first.
        let mut queue = BinaryHeap::new();
        // Pair type to positions where it currently occurs.
        let mut pairs = HashMap::new();

        for left in 0..nodes.len().saturating_sub(1) {
            self.add_pair(&nodes, &mut queue, &mut pairs, left);
        }

        while let Some(Reverse((_, pair))) = queue.pop() {
            // An earlier merge may have removed this pair.
            let Some(mut positions) = pairs.remove(&pair) else {
                continue;
            };
            positions.sort_unstable();

            // One BPE round merges all valid occurrences.
            for left in positions {
                let Some(right) = nodes[left].next else {
                    continue;
                };
                if !nodes[left].live
                    || !nodes[right].live
                    || (nodes[left].symbol, nodes[right].symbol) != pair
                {
                    continue;
                }

                let (_, result) = self.merge_ranks[&pair];
                let next = nodes[right].next;
                nodes[left].symbol = result;
                nodes[left].next = next;
                nodes[right].live = false;
                if let Some(next) = next {
                    nodes[next].previous = Some(left);
                }

                // Only pairs next to the merged node changed.
                if let Some(previous) = nodes[left].previous {
                    self.add_pair(&nodes, &mut queue, &mut pairs, previous);
                }
                self.add_pair(&nodes, &mut queue, &mut pairs, left);
            }
        }

        let mut output = Vec::new();
        let mut node = (!nodes.is_empty()).then_some(0);
        while let Some(index) = node {
            output.push(nodes[index].symbol);
            node = nodes[index].next;
        }
        output
    }

    fn add_pair(
        &self,
        nodes: &[Node],
        queue: &mut BinaryHeap<Reverse<(usize, (u32, u32))>>,
        pairs: &mut HashMap<(u32, u32), Vec<usize>>,
        left: usize,
    ) {
        let Some(right) = nodes[left].next else {
            return;
        };
        let pair = (nodes[left].symbol, nodes[right].symbol);
        let Some(&(rank, _)) = self.merge_ranks.get(&pair) else {
            return;
        };
        let positions = pairs.entry(pair).or_default();
        let first = positions.is_empty();
        positions.push(left);
        if first {
            queue.push(Reverse((rank, pair)));
        }
    }

    pub(crate) fn encode_with(
        &self,
        input: &str,
        mut encode_piece: impl FnMut(&str) -> Vec<u32>,
    ) -> Vec<u32> {
        self.pattern
            .find_iter(input)
            .flat_map(|matched| encode_piece(matched.unwrap().as_str()))
            .collect()
    }

    pub(crate) fn matches<'a>(&self, input: &'a str) -> Vec<&'a str> {
        self.pattern
            .find_iter(input)
            .map(|matched| matched.unwrap().as_str())
            .collect()
    }

    pub(crate) fn token_ids(&self, pieces: Vec<u32>) -> Vec<u32> {
        pieces
            .into_iter()
            .map(|piece| self.token_ids[&self.symbols[piece as usize]])
            .collect()
    }

    pub(crate) fn token_symbols(&self, tokens: &[u32]) -> String {
        tokens
            .iter()
            .map(|&token| self.token_symbols[token as usize].as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub(crate) fn byte_encode(&self, input: &str) -> String {
        input
            .bytes()
            .map(|byte| self.symbols[self.byte_ids[byte as usize] as usize].as_str())
            .collect()
    }

    pub(crate) fn parse_symbols(&self, merged: &str) -> Vec<u32> {
        merged
            .split(' ')
            .map(|symbol| self.symbol_ids[symbol])
            .collect()
    }

    pub(crate) fn symbols_to_string(&self, pieces: &[u32]) -> String {
        pieces
            .iter()
            .map(|&piece| self.symbols[piece as usize].as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

struct Node {
    symbol: u32,
    previous: Option<usize>,
    next: Option<usize>,
    live: bool,
}

fn merge_all(pieces: Vec<u32>, first: u32, second: u32, result: u32) -> Vec<u32> {
    let mut merged = Vec::with_capacity(pieces.len());
    let mut index = 0;
    while index < pieces.len() {
        if pieces[index] == first && pieces.get(index + 1) == Some(&second) {
            merged.push(result);
            index += 2;
        } else {
            merged.push(pieces[index]);
            index += 1;
        }
    }
    merged
}

pub(crate) fn tokenizer() -> &'static Tokenizer {
    TOKENIZER.get_or_init(|| {
        let mut ids = HashMap::new();
        let mut symbols = Vec::new();
        let mut byte_ids = [0; 256];
        for byte in 0..=u8::MAX {
            let symbol = byte_to_char(byte).to_string();
            byte_ids[byte as usize] = symbol_id(&mut ids, &mut symbols, &symbol);
        }

        let merge_ranks = MERGES
            .lines()
            .skip(1)
            .enumerate()
            .filter_map(|(rank, line)| {
                let mut pair = line.split_whitespace();
                let first = pair.next()?;
                let second = pair.next()?;
                let result = format!("{first}{second}");
                let first = symbol_id(&mut ids, &mut symbols, first);
                let second = symbol_id(&mut ids, &mut symbols, second);
                let result = symbol_id(&mut ids, &mut symbols, &result);
                // The source line number is the merge priority.
                Some(((first, second), (rank, result)))
            })
            .collect();

        let token_ids: HashMap<String, u32> = serde_json::from_str(VOCAB).unwrap();
        let mut token_symbols = vec![String::new(); token_ids.len()];
        for (symbol, &token) in &token_ids {
            token_symbols[token as usize] = symbol.clone();
        }

        Tokenizer {
            pattern: Regex::new(PATTERN).unwrap(),
            byte_ids,
            merge_ranks,
            symbols,
            symbol_ids: ids,
            token_ids,
            token_symbols,
        }
    })
}

pub(crate) fn byte_decoder() -> [u8; 512] {
    let mut output = [0; 512];
    for byte in 0..=u8::MAX {
        output[byte_to_char(byte) as usize] = byte;
    }
    output
}

fn symbol_id(ids: &mut HashMap<String, u32>, symbols: &mut Vec<String>, symbol: &str) -> u32 {
    if let Some(&id) = ids.get(symbol) {
        return id;
    }
    let id = symbols.len() as u32;
    ids.insert(symbol.to_owned(), id);
    symbols.push(symbol.to_owned());
    id
}

pub(crate) fn byte_to_char(byte: u8) -> char {
    match byte {
        b'!'..=b'~' | 0xA1..=0xAC | 0xAE..=0xFF => byte as char,
        _ => char::from_u32(256 + hidden_byte_index(byte)).unwrap(),
    }
}

fn hidden_byte_index(byte: u8) -> u32 {
    (0..byte)
        .filter(|value| !matches!(*value, b'!'..=b'~' | 0xA1..=0xAC | 0xAE..=0xFF))
        .count() as u32
}

#[cfg(test)]
mod tests {
    use super::{encode, encode_fast};

    #[test]
    fn matches_the_article_example() {
        assert_eq!(
            encode("what's the weather in goldshire?"),
            [10919, 338, 262, 6193, 287, 3869, 10932, 30],
        );
        assert_eq!(
            encode_fast("what's the weather in goldshire?"),
            encode("what's the weather in goldshire?"),
        );
    }
}
