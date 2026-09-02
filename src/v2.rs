use crate::baseline;

pub fn encode(input: &str) -> Vec<u32> {
    baseline::encode_fast(input)
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

    #[test]
    fn matches_on_generated_text() {
        let alphabet = b" abcdefghijklmnopqrstuvwxyz0123456789!?-'\n";
        let mut state = 1_u32;
        for _ in 0..100 {
            let mut text = String::new();
            for _ in 0..64 {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                text.push(alphabet[state as usize % alphabet.len()] as char);
            }
            assert_eq!(encode(&text), baseline::encode(&text));
        }
    }
}
