"""Build the small English frequency asset used by the constellation.

Run from the repository root after npm install in client:
uv run --isolated --no-project --with wordfreq==3.1.1 python client/scripts/build-word-frequencies.py
"""

import json
from collections import defaultdict
from importlib.metadata import version
from pathlib import Path

from wordfreq import zipf_frequency

client = Path(__file__).resolve().parents[1]
words = set()
for path in (client / "node_modules/wordlist-english").glob("*-words-*.json"):
    if int(path.stem.rsplit("-", 1)[1]) <= 50:
        words.update(word.lower() for word in json.loads(path.read_text()))

bins = defaultdict(list)
for word in sorted(words):
    score = round(zipf_frequency(word, "en") * 100)
    if score:
        bins[score].append(word)

asset = {
    "source": "wordfreq",
    "version": version("wordfreq"),
    "author": "Robyn Speer",
    "url": "https://github.com/rspeer/wordfreq",
    "license": "CC-BY-SA-4.0",
    "note": "English Zipf scores, multiplied by 100; usage snapshot through about 2021. Subset: SCOWL <=50. See README.md for source credits.",
    "bins": {str(score): "\n".join(bins[score]) for score in sorted(bins, reverse=True)},
}
destination = client / "data/english-frequencies.json"
destination.parent.mkdir(exist_ok=True)
destination.write_text(json.dumps(asset, ensure_ascii=False, separators=(",", ":")) + "\n")
print(f"{sum(map(len, bins.values()))} word scores, {destination.stat().st_size} bytes")
