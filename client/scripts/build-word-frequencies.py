"""Build the small English frequency asset used by the constellation.

Run from the repository root after npm install in client:
uv run --isolated --no-project --with wordfreq==3.1.1 python client/scripts/build-word-frequencies.py
"""

import json
from collections import defaultdict
from importlib.metadata import version
from pathlib import Path

from wordfreq import zipf_frequency

# The SCOWL tier scored up to. It must cover the constellation's default
# word list (COMMON_LIMIT in src/constellation/branches.ts); the client build
# refuses an asset whose "scowl_limit" is lower.
SCOWL_LIMIT = 50

client = Path(__file__).resolve().parents[1]
words = set()
for path in (client / "node_modules/wordlist-english").glob("*-words-*.json"):
    if int(path.stem.rsplit("-", 1)[1]) <= SCOWL_LIMIT:
        words.update(word.lower() for word in json.loads(path.read_text()))
if not words:
    raise SystemExit("no SCOWL word lists found: run npm install in client first")

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
    "note": f"English Zipf scores, multiplied by 100; usage snapshot through about 2021. Subset: SCOWL <={SCOWL_LIMIT}. See README.md for source credits.",
    "scowl_limit": SCOWL_LIMIT,
    "bins": {str(score): "\n".join(bins[score]) for score in sorted(bins, reverse=True)},
}
destination = client / "data/english-frequencies.json"
destination.parent.mkdir(exist_ok=True)
destination.write_text(json.dumps(asset, ensure_ascii=False, separators=(",", ":")) + "\n")
print(f"{sum(map(len, bins.values()))} word scores, {destination.stat().st_size} bytes")
