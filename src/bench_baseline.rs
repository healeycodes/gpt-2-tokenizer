use std::env;
use std::fs;
use std::time::Instant;

use gpt2_tokenizer::baseline;

const SAMPLES: usize = 9;

fn main() {
    let encode = if env::args().any(|argument| argument == "--fast") {
        gpt2_tokenizer::v2::encode
    } else if env::args().any(|argument| argument == "--cache-first") {
        gpt2_tokenizer::v3::encode_cache_first
    } else if env::args().any(|argument| argument == "--tiktoken") {
        gpt2_tokenizer::scratch_tiktoken::encode
    } else if env::args().any(|argument| argument == "--no-fast-path") {
        gpt2_tokenizer::v3::encode_no_fast_path
    } else if env::args().any(|argument| argument == "--std-hash") {
        gpt2_tokenizer::v3::encode_std_hash
    } else if env::args().any(|argument| argument == "--bytes") {
        gpt2_tokenizer::v3::encode
    } else {
        baseline::encode
    };
    let moby_dick = fs::read_to_string("inputs/bench/moby-dick-32k.txt").unwrap();
    let react = fs::read_to_string("inputs/bench/react-19.2.0-8m.js").unwrap();
    let long_piece = fs::read_to_string("inputs/bench/long-a-4k.txt").unwrap();
    bench("Moby-Dick", &moby_dick, encode);
    bench("React", &react, encode);
    if env::args().any(|argument| argument == "--long-piece") {
        bench("Long a", &long_piece, encode);
    }
}

fn bench(name: &str, input: &str, encode: fn(&str) -> Vec<u32>) {
    let _ = encode(input);
    let mut times = (0..SAMPLES)
        .map(|_| {
            let start = Instant::now();
            let _ = encode(input);
            start.elapsed()
        })
        .collect::<Vec<_>>();
    times.sort();
    let time = times[SAMPLES / 2];

    println!(
        "{name:13} {size:7} KiB  {ms:8.2} ms  {mibps:6.1} MiB/s",
        size = input.len() / 1024,
        ms = time.as_secs_f64() * 1_000.0,
        mibps = input.len() as f64 / time.as_secs_f64() / 1_048_576.0,
    );
}
