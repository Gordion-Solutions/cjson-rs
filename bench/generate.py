#!/usr/bin/env python3
"""Generate the benchmark documents (8-43 MB each) into a directory.

Usage: generate.py <out-dir>
"""
import json
import os
import random
import sys

out = sys.argv[1]
os.makedirs(out, exist_ok=True)
random.seed(1)
words = "the quick brown fox jumps over lazy dog lorem ipsum dolor sit amet consectetur".split()


def text(n):
    return " ".join(random.choice(words) for _ in range(n))


# Records: many small objects mixing every value type (exports, API payloads).
with open(f"{out}/records.json", "w") as f:
    f.write("[")
    for i in range(300_000):
        rec = {"id": i, "name": text(3), "active": bool(i % 3), "score": i * 1.5,
               "tags": [f"t{i % 7}", f"t{i % 11}"], "owner": None,
               "address": {"city": text(1), "zip": f"{i % 99999:05d}"}}
        f.write(("," if i else "") + json.dumps(rec, separators=(",", ":")))
    f.write("]")

# Numbers: a long array of integers and fractions (sensor data, coordinates).
with open(f"{out}/numbers.json", "w") as f:
    f.write("[")
    for i in range(3_000_000):
        v = i if i % 3 == 0 else round(random.uniform(-1e4, 1e4), 6)
        f.write(("," if i else "") + json.dumps(v))
    f.write("]")

# Strings: text-heavy, with escapes and non-ASCII.
with open(f"{out}/strings.json", "w") as f:
    f.write("[")
    for i in range(120_000):
        s = text(40) + ' "quoted" \\ back\\slash\n\ttab éü中文 \U0001F600'
        f.write(("," if i else "") + json.dumps({"k": i, "text": s}, separators=(",", ":")))
    f.write("]")

# Nested: deep arrays and objects (within cJSON's nesting limit of 1000).
def nest(depth):
    return "[" * depth + '"leaf"' + "]" * depth
with open(f"{out}/nested.json", "w") as f:
    f.write("[")
    for i in range(20_000):
        d = 100 + i % 200
        f.write(("," if i else "") + nest(d))
    f.write("]")
