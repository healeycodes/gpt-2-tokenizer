# BPE token-counting fixtures

This directory contains exactly one compact input fixture for each common BPE
token-counting workload. All prose, code, structured data, logs, and generated
strings are self-authored synthetic material. No external text or downloads are
included. Files are UTF-8 unless noted otherwise. `base64-like.txt` and
`random-ascii.txt` are printable ASCII, which is also valid UTF-8.

| File | Workload and tokenizer behavior exercised |
| --- | --- |
| `english-prose.txt` | Natural English words, contractions/quotes, sentence punctuation, paragraphs, and ordinary whitespace. |
| `source-code.js` | JavaScript syntax, indentation, comments, identifiers, camel case, literals, operators, and delimiters. |
| `structured-data.json` | Valid JSON with repeated keys, quoted strings, escapes, IDs, booleans, `null`, arrays, numbers, and nesting. |
| `application.log` | Timestamped log records, levels, key-value fields, paths, identifiers, quoted errors, and line-oriented records. |
| `cjk-text.txt` | Chinese, Japanese, and Korean text with CJK punctuation and mixed Latin/numeric content. |
| `emoji-mixed-unicode.txt` | Accented text, multiple scripts, symbols, emoji, variation selectors, skin tones, ZWJ sequences, and flags. |
| `base64-like.txt` | Base64-style printable ASCII, including `+`, `/`, and `=`. Useful for binary-encoded payload behavior. |
| `repeated-boundaries.txt` | Long runs, repeated substrings, case changes, delimiters, doubled separators, and line/end boundaries. |
| `random-ascii.txt` | Fixed deterministic random-like printable ASCII with broad punctuation coverage and low repetition. |

## Scaling guidance

For large benchmarks, repeat the exact bytes of a fixture rather than
regenerating or normalizing it. Concatenate complete copies with the separator
already natural to that workload: a blank line for prose/CJK/Unicode, a newline
for code, JSON must instead be wrapped as valid independent records or measured
as concatenated raw bytes intentionally, and a newline for logs, base64-like,
and random ASCII. Preserve the final newline status of each copy when studying
boundary effects. Do not insert arbitrary spaces, trim trailing newlines, split
UTF-8 code points or emoji grapheme sequences, fold base64 lines differently,
or add headers. Each change can alter BPE merges and therefore token counts.

For `repeated-boundaries.txt`, benchmark both exact whole-file repetition and
explicit separator variants as separate cases. Its purpose is to expose
cross-copy merges and start/end behavior, so an added newline or blank line is
itself a material test condition.
