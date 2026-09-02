use std::env;
use std::fs;

use gpt2_tokenizer::baseline;

fn main() {
    let fast = env::args().any(|argument| argument == "--fast");
    let bytes = env::args().any(|argument| argument == "--bytes");
    let no_fast_path = env::args().any(|argument| argument == "--no-fast-path");
    let std_hash = env::args().any(|argument| argument == "--std-hash");
    let cache_first = env::args().any(|argument| argument == "--cache-first");
    let tiktoken = env::args().any(|argument| argument == "--tiktoken");
    let path = env::args()
        .skip(1)
        .find(|argument| !argument.starts_with("--"))
        .expect("input path");
    let input = fs::read_to_string(path).expect("UTF-8 input");
    let tokens = if cache_first {
        gpt2_tokenizer::v3::encode_cache_first(&input)
    } else if tiktoken {
        gpt2_tokenizer::scratch_tiktoken::encode(&input)
    } else if std_hash {
        gpt2_tokenizer::v3::encode_std_hash(&input)
    } else if no_fast_path {
        gpt2_tokenizer::v3::encode_no_fast_path(&input)
    } else if bytes {
        gpt2_tokenizer::v3::encode(&input)
    } else if fast {
        baseline::encode_fast(&input)
    } else {
        baseline::encode(&input)
    };
    println!("{tokens:?}");
}
