use std::sync::OnceLock;

use tiktoken_rs::{CoreBPE, r50k_base};

static BPE: OnceLock<CoreBPE> = OnceLock::new();

pub fn encode(input: &str) -> Vec<u32> {
    bpe()
        .encode_ordinary(input)
        .into_iter()
        .map(|token| token as u32)
        .collect()
}

fn bpe() -> &'static CoreBPE {
    BPE.get_or_init(|| r50k_base().unwrap())
}

#[cfg(test)]
mod tests {
    use crate::baseline;

    use super::encode;

    #[test]
    fn matches_the_baseline() {
        let text = "what's the weather in goldshire?";
        assert_eq!(encode(text), baseline::encode(text));
    }
}
