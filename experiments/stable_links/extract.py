"""Extract per-entry identity fields from wety's entries (`wiktionary_entries`).

Usage: python extract.py ENTRIES.jsonl.gz REDIRECTS.tsv.gz OUT.tsv
Columns: kind lang word ety pos canon senseids gloss etytext etymon_id digest
kind is E (one part of speech of an entry) or R (redirect: lang='' word=title
canon=target). word is the page title, a reconstruction's without its
"Reconstruction:<language>/" prefix; ety the "Etymology N" label (empty when
unnumbered); canon the first headword where it is not the title. etymon_id is the
id= argument of the etymology's first {{etymon}} or {{ety}} call, if any; digest the
etymology section's digest, unchanged while its content is.
"""

import json
import subprocess
import sys


def clean(s, n):
    return (s or "")[:n].replace("\t", " ").replace("\n", " ")


def lines(path):
    proc = subprocess.Popen(["pigz", "-dc", path], stdout=subprocess.PIPE, bufsize=1 << 20)
    yield from proc.stdout


def title(page, ns):
    if ns == 118 and "/" in page:
        return page.split("/", 1)[1]
    return page


def main(entries, redirects, out):
    with open(out, "w", encoding="utf-8") as f:
        for line in lines(redirects):
            fields = line.decode("utf-8").rstrip("\n").split("\t")
            if len(fields) != 3:
                continue
            ns, page, target = fields
            ns = int(ns) if ns.isdigit() else 0
            f.write(
                f"R\t\t{clean(title(page, ns), 200)}\t\t\t{clean(title(target, ns), 200)}\t\t\t\t\t\n"
            )
        for line in lines(entries):
            e = json.loads(line)
            lang = e.get("lang")
            if not lang:
                continue
            word = title(e["page"], e.get("ns", 0))
            ety = e.get("ety_label") or ""
            section = e.get("etymology") or {}
            blocks = section.get("blocks") or []
            etytext = " ".join(b.get("text") or "" for b in blocks).strip()
            etymon = ""
            for b in blocks:
                for s in b.get("spans") or ():
                    for c in s.get("calls") or ():
                        if c.get("name") in ("etymon", "ety") and (c.get("args") or {}).get("id"):
                            etymon = etymon or c["args"]["id"]
            for p in e.get("pos") or ():
                heads = [h.lstrip("*") for h in p.get("headword") or ()]
                canon = heads[0] if heads and heads[0] != word else ""
                senses = p.get("senses") or []
                sids = sorted({s for sense in senses for s in sense.get("senseids") or ()})
                gloss = senses[0].get("gloss") or "" if senses else ""
                f.write(
                    "\t".join(
                        [
                            "E",
                            lang,
                            clean(word, 200),
                            ety,
                            p.get("pos") or "",
                            clean(canon, 200),
                            clean("|".join(sids), 300),
                            clean(gloss, 100),
                            clean(etytext, 300),
                            clean(etymon, 100),
                            section.get("digest") or "",
                        ]
                    )
                    + "\n"
                )


if __name__ == "__main__":
    main(*sys.argv[1:])
