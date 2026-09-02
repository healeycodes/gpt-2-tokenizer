use std::env;
use std::fs;
use std::time::Duration;

use gpt2_tokenizer::{baseline, v3};

fn main() {
    let version = env::args().nth(1).expect("version: v1, v2, or v3");
    let path = env::args().nth(2).expect("input path");
    let input = fs::read_to_string(path).expect("UTF-8 input");
    let profile = match version.as_str() {
        "v1" => baseline::profile(&input, false),
        "v2" => baseline::profile(&input, true),
        "v3" => v3::profile(&input),
        _ => panic!("version: v1, v2, or v3"),
    };
    let total = profile.regex + profile.bytes + profile.cache + profile.merge + profile.output;
    for (name, time) in [
        ("regex", profile.regex),
        ("bytes", profile.bytes),
        ("cache", profile.cache),
        ("merge", profile.merge),
        ("output", profile.output),
    ] {
        print(name, time, total);
    }
}

fn print(name: &str, time: Duration, total: Duration) {
    let percent = time.as_secs_f64() / total.as_secs_f64() * 100.0;
    println!(
        "{name:7} {percent:5.1}%  {:8.3} ms",
        time.as_secs_f64() * 1_000.0
    );
}
