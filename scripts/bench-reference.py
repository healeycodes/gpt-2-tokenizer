import json
import statistics
import sys
import time
from collections import OrderedDict
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scratch"))
import encoder

SAMPLES = 9


class Cache(OrderedDict):
    def __getitem__(self, key):
        value = super().__getitem__(key)
        self.move_to_end(key)
        return value

    def __setitem__(self, key, value):
        if key in self:
            self.move_to_end(key)
        super().__setitem__(key, value)
        if len(self) > 256:
            self.popitem(last=False)


def tokenizer():
    with open(ROOT / "assets/encoder.json") as file:
        vocabulary = json.load(file)
    with open(ROOT / "assets/vocab.bpe", encoding="utf-8") as file:
        merges = [tuple(line.split()) for line in file.read().splitlines()[1:-1]]
    output = encoder.Encoder(vocabulary, merges)
    output.cache = Cache()
    return output


def bench(name, input):
    tokenizer_instance = tokenizer()
    tokenizer_instance.encode(input)
    samples = []
    for _ in range(SAMPLES):
        tokenizer_instance.cache.clear()
        start = time.perf_counter()
        tokenizer_instance.encode(input)
        samples.append(time.perf_counter() - start)
    seconds = statistics.median(samples)
    mibps = len(input.encode()) / seconds / 1_048_576
    print(f"{name:13} {seconds * 1_000:8.2f} ms  {mibps:6.1f} MiB/s")


bench("Moby-Dick", (ROOT / "inputs/bench/moby-dick-32k.txt").read_text())
bench("React", (ROOT / "inputs/bench/react-19.2.0-8m.js").read_text())
