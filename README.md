# 🪙 gpt-2-tokenizer
> My blog post: [What Makes LLM Tokenization Slow?](https://healeycodes.com/what-makes-llm-tokenization-slow/)

<br>

A small Rust port of GPT-2's tokenizer and a few faster variants.

```text
./scripts/prepare-bench
./scripts/get-reference
./scripts/test
./scripts/compare-python
./scripts/bench-final
```

`prepare-bench` downloads Moby-Dick and React source for the benchmarks.

The Python comparison needs Python 3 and `regex`:

```text
python3 -m venv .venv
.venv/bin/pip install regex
```

The tokenizer matches GPT-2 `encoder.py` at commit:

```text
9b63575ef42771a015060c964af2c3da4cf7c8ab
```

Included GPT-2 asset SHA-256 values:

```text
encoder.json
196139668be63f3b5d6574427317ae82f612a97c5d1cdaf36ed2256dbf636783

vocab.bpe
1ce1664773c50f3e0cc8842619a93edc4624525b728b188a9e0be33b7726adc5
```
