"""Measure how well (lang, page, Etymology N) identifies an etymology across two dumps.

Inputs are extract.py outputs for an older and a newer dump. A page is a
(language, page title) section; an etymology is one Etymology N within it.
Etymologies are matched across dumps by etymology text, gloss and POS overlap.

Usage: python analyze.py old.tsv new.tsv
"""

import random
import re
import sys
from collections import Counter, defaultdict

TOK = re.compile(r"\w+", re.UNICODE)


MULTI = "\x00"


def scan(path):
    """Pass 1: page -> its only etymology number, or MULTI; redirect titles; canon collisions."""
    keys = {}
    redirects = set()
    canon_first = {}
    canon_multi = 0
    with open(path, encoding="utf-8") as f:
        for line in f:
            p = line.split("\t", 6)
            if len(p) < 7:
                continue
            kind, lang, word, ety, _pos, canon = p[:6]
            if kind == "R":
                redirects.add(word)
                continue
            k = lang + "\t" + word
            prev = keys.get(k)
            if prev is None:
                keys[k] = ety
            elif prev != ety and prev != MULTI:
                keys[k] = MULTI
            if canon and canon != word:
                ck = lang + "\t" + canon
                w0 = canon_first.setdefault(ck, word)
                if w0 != word and w0 != MULTI:
                    canon_first[ck] = MULTI
                    canon_multi += 1
    return keys, redirects, canon_multi


def load_groups(path, wanted):
    """Pass 2: full etymology groups for the wanted pages only."""
    pages = defaultdict(dict)
    with open(path, encoding="utf-8") as f:
        for line in f:
            p = line.rstrip("\n").split("\t")
            if len(p) < 10 or p[0] != "E":
                continue
            _, lang, word, ety, pos, _canon, sids, gloss, etytext, etymon = p[:10]
            k = lang + "\t" + word
            if k not in wanted:
                continue
            g = pages[k].get(ety)
            if g is None:
                g = pages[k][ety] = {"pos": [], "gloss": [], "ety": "", "sids": set(), "etymon": ""}
            g["pos"].append(pos)
            if gloss:
                g["gloss"].append(gloss)
            if etytext and not g["ety"]:
                g["ety"] = etytext
            if sids:
                g["sids"].update(sids.split("|"))
            if etymon and not g["etymon"]:
                g["etymon"] = etymon
    return pages


def toks(s):
    return set(TOK.findall(s.lower()))


def jacc(a, b):
    if not a and not b:
        return None
    return len(a & b) / len(a | b)


def sim(a, b):
    je = jacc(toks(a["ety"]), toks(b["ety"]))
    jg = jacc(toks(" ".join(a["gloss"])), toks(" ".join(b["gloss"])))
    jp = jacc(set(a["pos"]), set(b["pos"]))
    parts = [(x, w) for x, w in ((je, 0.5), (jg, 0.35), (jp, 0.15)) if x is not None]
    if not parts:
        return 0.0
    return sum(x * w for x, w in parts) / sum(w for _, w in parts)


def pct(a, b):
    return f"{a:,}/{b:,} ({100 * a / b:.2f}%)" if b else f"{a}/0"


def is_en(k):
    return k.startswith("en\t")


def prevalence(name, keys, canon_multi, pages):
    print(f"\n## Homographs in {name}")
    for label, filt in (("all languages", lambda k: True), ("English", is_en)):
        n = m = 0
        for k, v in keys.items():
            if filt(k):
                n += 1
                m += v == MULTI
        groups_multi = sum(len(e) for k, e in pages.items() if keys.get(k) == MULTI and filt(k))
        print(
            f"- {label}: pages (lang sections) with >1 etymology: {pct(m, n)}; "
            f"etymology groups on such pages: {groups_multi:,}"
        )
    roots = sum(1 for k, e in pages.items() for ety, g in e.items() if g["pos"].count("root") > 1)
    print(
        f"- pages with several Root sections sharing one etymology number (wety renumbers): {roots:,}"
    )
    print(f"- (lang, canonical form != page) reached from >1 page: {canon_multi:,}")


def drift(kold, knew, rnew, old, new):
    print("\n## Old dump -> new dump, per (lang, page)")
    for label, filt in (("all", lambda k: True), ("en", is_en)):
        n_old = n_both = gone = redirected = 0
        c = Counter()
        for k, v in kold.items():
            if not filt(k):
                continue
            n_old += 1
            nv = knew.get(k)
            if nv is None:
                gone += 1
                redirected += k.split("\t", 1)[1] in rnew
                continue
            n_both += 1
            c[(v == MULTI, nv == MULTI)] += 1
        print(
            f"- [{label}] pages in old dump {n_old:,}; still present {pct(n_both, n_old)}; gone {gone:,} "
            f"(title now a redirect: {redirected:,})"
        )
        print(
            f"  [{label}] single->single {c[(False, False)]:,}, single->multi {c[(False, True)]:,}, "
            f"multi->single {c[(True, False)]:,}, multi->multi {c[(True, True)]:,}"
        )
        groups = same = renum = unmatched = silent = 0
        tg = tsilent = 0
        ek = ec = el = 0
        samples = defaultdict(list)
        for k, o in old.items():
            if not filt(k) or kold.get(k) != MULTI or k not in new:
                continue
            n = new[k]
            for ety, og in o.items():
                groups += 1
                tg += bool(og["ety"])
                best, _, be = max((sim(og, ng), ne == ety, ne) for ne, ng in n.items())
                if best < 0.35:
                    unmatched += 1
                    samples["unmatched"].append((k, ety))
                    continue
                if be == ety:
                    same += 1
                else:
                    renum += 1
                    samples["renumbered"].append((k, ety, be, round(best, 2)))
                    silent += ety in n
                    tsilent += bool(og["ety"]) and ety in n
                eo, en_ = og["etymon"], n[be]["etymon"]
                if eo:
                    if en_ == eo:
                        ek += 1
                    elif en_:
                        ec += 1
                    else:
                        el += 1
        print(
            f"  [{label}] etymologies on old multi-etymology pages still present: {groups:,}: "
            f"same number {pct(same, groups)}, renumbered {pct(renum, groups)} "
            f"(old number now names another etymology: {silent:,}), no confident match {pct(unmatched, groups)}"
        )
        print(
            f"  [{label}] of these with etymology text: {tg:,}; old number now names another etymology: {pct(tsilent, tg)}"
        )
        print(
            f"  [{label}] etymon ids on matched etymologies: kept {ek:,}, changed {ec:,}, dropped {el:,}"
        )
        random.seed(1)
        for kind in ("renumbered", "unmatched"):
            s = samples[kind]
            print(f"  [{label}] sample {kind}: {random.sample(s, min(10, len(s)))}")


def ids_coverage(name, keys, pages):
    print(f"\n## Identifier coverage on multi-etymology pages, {name}")
    for label, filt in (("all", lambda k: True), ("en", is_en)):
        groups = with_sid = with_etymon = all_e = all_s = npages = 0
        for k, e in pages.items():
            if keys.get(k) != MULTI or not filt(k):
                continue
            npages += 1
            ae = as_ = True
            for g in e.values():
                groups += 1
                if g["sids"]:
                    with_sid += 1
                else:
                    as_ = False
                if g["etymon"]:
                    with_etymon += 1
                else:
                    ae = False
            all_e += ae
            all_s += as_
        print(
            f"- [{label}] etymologies with a senseid {pct(with_sid, groups)}; with an etymon id "
            f"{pct(with_etymon, groups)}; pages where every etymology has an etymon id {pct(all_e, npages)}; "
            f"every etymology has a senseid {pct(all_s, npages)}"
        )


def main(old_p, new_p):
    kold, _, _ = scan(old_p)
    knew, rnew, cmnew = scan(new_p)
    wanted = {k for k, v in kold.items() if v == MULTI} | {k for k, v in knew.items() if v == MULTI}
    old = load_groups(old_p, wanted)
    new = load_groups(new_p, wanted)
    prevalence("new dump", knew, cmnew, new)
    drift(kold, knew, rnew, old, new)
    ids_coverage("old dump", kold, old)
    ids_coverage("new dump", knew, new)


if __name__ == "__main__":
    main(*sys.argv[1:])
