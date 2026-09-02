use std::collections::HashMap;
use std::hash::BuildHasher;
use std::sync::OnceLock;

use rustc_hash::FxHashMap;

use crate::baseline;
use crate::cache::BpeCache;
use crate::profile::{Profile, time};

static FX_RANKS: OnceLock<FxHashMap<Vec<u8>, u32>> = OnceLock::new();
static STD_RANKS: OnceLock<HashMap<Vec<u8>, u32>> = OnceLock::new();

pub fn encode(input: &str) -> Vec<u32> {
    encode_with(input, fx_ranks(), true, false)
}

pub fn encode_no_fast_path(input: &str) -> Vec<u32> {
    encode_with(input, fx_ranks(), false, false)
}

pub fn encode_std_hash(input: &str) -> Vec<u32> {
    encode_with(input, std_ranks(), true, false)
}

pub fn encode_cache_first(input: &str) -> Vec<u32> {
    encode_with(input, fx_ranks(), true, true)
}

pub fn profile(input: &str) -> Profile {
    let tokenizer = baseline::tokenizer();
    let mut cache = BpeCache::default();
    let mut profile = Profile::default();
    let matches = time(&mut profile.regex, || tokenizer.matches(input));
    let mut output = Vec::new();
    for piece in matches {
        output.extend(encode_piece_profile(
            piece,
            fx_ranks(),
            &mut cache,
            &mut profile,
        ));
    }
    profile
}

fn encode_with<S: BuildHasher>(
    input: &str,
    ranks: &HashMap<Vec<u8>, u32, S>,
    fast_path: bool,
    cache_first: bool,
) -> Vec<u32> {
    let tokenizer = baseline::tokenizer();
    let mut cache = BpeCache::default();
    tokenizer.encode_with(input, |piece| {
        encode_piece_with(piece, ranks, fast_path, cache_first, &mut cache)
    })
}

fn encode_piece_with<S: BuildHasher>(
    piece: &str,
    ranks: &HashMap<Vec<u8>, u32, S>,
    fast_path: bool,
    cache_first: bool,
    cache: &mut BpeCache,
) -> Vec<u32> {
    let bytes = piece.as_bytes();
    let encoded = baseline::tokenizer().byte_encode(piece);
    if cache_first && let Some(merged) = cache.get(&encoded) {
        return baseline::tokenizer().token_ids(baseline::tokenizer().parse_symbols(&merged));
    }
    // A learned token needs no BPE work at all.
    if fast_path && let Some(&token) = ranks.get(bytes) {
        if bytes.len() > 1 {
            cache.put(encoded, byte_symbols(bytes));
        }
        return vec![token];
    }
    if !cache_first && let Some(merged) = cache.get(&encoded) {
        return baseline::tokenizer().token_ids(baseline::tokenizer().parse_symbols(&merged));
    }
    if bytes.len() == 1 {
        return vec![ranks[bytes]];
    }

    // Parts are offsets into the unchanged input slice.
    let mut parts = (0..=bytes.len())
        .map(|start| Part {
            start,
            rank: if start + 2 <= bytes.len() {
                rank(ranks, &bytes[start..start + 2])
            } else {
                u32::MAX
            },
        })
        .collect::<Vec<_>>();

    while let Some((index, _)) = parts
        .iter()
        .enumerate()
        .min_by_key(|(_, part)| part.rank)
        .filter(|(_, part)| part.rank != u32::MAX)
    {
        // Only ranks next to this removed boundary can change.
        if index > 0 {
            parts[index - 1].rank = merged_rank(ranks, bytes, &parts, index - 1);
        }
        parts[index].rank = merged_rank(ranks, bytes, &parts, index);
        parts.remove(index + 1);
    }

    let symbols = parts
        .windows(2)
        .map(|pair| byte_symbols(&bytes[pair[0].start..pair[1].start]))
        .collect::<Vec<_>>();
    cache.put(encoded, symbols.join(" "));
    parts
        .windows(2)
        .map(|pair| ranks[&bytes[pair[0].start..pair[1].start]])
        .collect()
}

fn encode_piece_profile(
    piece: &str,
    ranks: &FxHashMap<Vec<u8>, u32>,
    cache: &mut BpeCache,
    profile: &mut Profile,
) -> Vec<u32> {
    let bytes = piece.as_bytes();
    let encoded = time(&mut profile.bytes, || {
        baseline::tokenizer().byte_encode(piece)
    });
    let cached = time(&mut profile.cache, || cache.get(&encoded));
    if let Some(merged) = cached {
        return time(&mut profile.output, || {
            baseline::tokenizer().token_ids(baseline::tokenizer().parse_symbols(&merged))
        });
    }
    if let Some(token) = time(&mut profile.cache, || ranks.get(bytes).copied()) {
        if bytes.len() > 1 {
            cache.put(encoded, byte_symbols(bytes));
        }
        return vec![token];
    }
    let tokens = time(&mut profile.merge, || encode_offsets(bytes, ranks));
    cache.put(encoded, baseline::tokenizer().token_symbols(&tokens));
    time(&mut profile.output, || tokens)
}

fn encode_offsets<S: BuildHasher>(bytes: &[u8], ranks: &HashMap<Vec<u8>, u32, S>) -> Vec<u32> {
    if bytes.len() == 1 {
        return vec![ranks[bytes]];
    }
    let mut parts = (0..=bytes.len())
        .map(|start| Part {
            start,
            rank: if start + 2 <= bytes.len() {
                rank(ranks, &bytes[start..start + 2])
            } else {
                u32::MAX
            },
        })
        .collect::<Vec<_>>();
    while let Some((index, _)) = parts
        .iter()
        .enumerate()
        .min_by_key(|(_, part)| part.rank)
        .filter(|(_, part)| part.rank != u32::MAX)
    {
        if index > 0 {
            parts[index - 1].rank = merged_rank(ranks, bytes, &parts, index - 1);
        }
        parts[index].rank = merged_rank(ranks, bytes, &parts, index);
        parts.remove(index + 1);
    }
    parts
        .windows(2)
        .map(|pair| ranks[&bytes[pair[0].start..pair[1].start]])
        .collect()
}

struct Part {
    start: usize,
    rank: u32,
}

fn merged_rank<S: BuildHasher>(
    ranks: &HashMap<Vec<u8>, u32, S>,
    bytes: &[u8],
    parts: &[Part],
    index: usize,
) -> u32 {
    if index + 3 >= parts.len() {
        return u32::MAX;
    }
    rank(ranks, &bytes[parts[index].start..parts[index + 3].start])
}

fn rank<S: BuildHasher>(ranks: &HashMap<Vec<u8>, u32, S>, bytes: &[u8]) -> u32 {
    // GPT-2 assigns merged-token IDs in BPE merge order.
    ranks.get(bytes).copied().unwrap_or(u32::MAX)
}

fn fx_ranks() -> &'static FxHashMap<Vec<u8>, u32> {
    FX_RANKS.get_or_init(|| vocabulary().into_iter().collect())
}

fn std_ranks() -> &'static HashMap<Vec<u8>, u32> {
    STD_RANKS.get_or_init(|| vocabulary().into_iter().collect())
}

fn byte_symbols(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&byte| baseline::byte_to_char(byte))
        .collect()
}

fn vocabulary() -> Vec<(Vec<u8>, u32)> {
    let ids: HashMap<String, u32> =
        serde_json::from_str(include_str!("../assets/encoder.json")).unwrap();
    let decode = baseline::byte_decoder();
    ids.into_iter()
        .map(|(symbol, id)| {
            // Undo GPT-2's byte-to-Unicode representation once.
            let bytes = symbol
                .chars()
                .map(|symbol| decode[symbol as usize])
                .collect();
            (bytes, id)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::baseline;

    use super::{encode, encode_no_fast_path, encode_std_hash};

    #[test]
    fn matches_the_baseline() {
        let text = "what's the weather in goldshire?";
        let expected = baseline::encode(text);
        assert_eq!(encode(text), expected);
        assert_eq!(encode_no_fast_path(text), expected);
        assert_eq!(encode_std_hash(text), expected);
    }
}
