"""Extract per-entry identity fields from a Wiktextract raw dump.

Usage: python extract.py DUMP.jsonl.gz OUT.tsv
Columns: kind lang word ety pos canon senseids gloss etytext etymon_id
kind is E (entry) or R (redirect: lang='' word=title canon=target).
etymon_id is the id= argument of the entry's first {{etymon}} template, if any.
"""

import json
import subprocess
import sys


def clean(s, n):
    return (s or "")[:n].replace("\t", " ").replace("\n", " ")


def main(dump, out):
    proc = subprocess.Popen(["pigz", "-dc", dump], stdout=subprocess.PIPE, bufsize=1 << 20)
    with open(out, "w", encoding="utf-8") as f:
        for line in proc.stdout:
            j = json.loads(line)
            if "redirect" in j:
                f.write(
                    f"R\t\t{clean(j.get('title'), 200)}\t\t\t{clean(j.get('redirect'), 200)}\t\t\t\t\n"
                )
                continue
            word = j.get("word")
            lang = j.get("lang_code")
            if not word or not lang:
                continue
            ety = j.get("etymology_number")
            ety = "" if ety is None else str(ety)
            canon = ""
            for form in j.get("forms") or ():
                if "canonical" in (form.get("tags") or ()):
                    canon = form.get("form") or ""
                    break
            senses = j.get("senses") or []
            sids = []
            for s in senses:
                sid = s.get("senseid")
                if isinstance(sid, list):
                    sids.extend(sid)
                elif sid:
                    sids.append(sid)
            etymon = ""
            for t in j.get("etymology_templates") or ():
                if t.get("name") == "etymon" and (t.get("args") or {}).get("id"):
                    etymon = t["args"]["id"]
                    break
            gloss = ""
            if senses and senses[0].get("glosses"):
                gloss = senses[0]["glosses"][0]
            f.write(
                "\t".join(
                    [
                        "E",
                        lang,
                        clean(word, 200),
                        ety,
                        j.get("pos") or "",
                        clean(canon, 200),
                        clean("|".join(sorted(set(map(str, sids)))), 300),
                        clean(gloss, 100),
                        clean(j.get("etymology_text"), 300),
                        clean(etymon, 100),
                    ]
                )
                + "\n"
            )


if __name__ == "__main__":
    main(*sys.argv[1:])
