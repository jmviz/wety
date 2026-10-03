"""Command line entry point.

dating bootstrap [glottolog|wikidata|wikipedia|iecor|dplace|asjp|curated|all]
dating synthesize [--family GLOTTOCODE]      validate and publish the complete evidence generation
dating tasks                                 write data/_derived/tasks.json (ranked task list)
dating discover FAMILY [--max N] [--screen llm|keyword|none]   search + screening for one task
dating retrieve FAMILY                       full texts for included papers
dating extract FAMILY [--batch]              LLM extraction -> claims (--batch: OpenRouter batch API, 50% price)
dating batch-poll FAMILY                     ingest finished batches
dating llm-check [BACKEND ...]               same structured probe through every backend (parity check)
dating costs                                 model spend and tokens from the logs
dating llm-synthesize FAMILY                 save a review proposal; never adopt model dates
dating validate                              check current generation, provenance, dependencies and mirrors
dating export-periods [--out PATH]           display periods from adopted decisions -> language_periods.json
dating report
"""

from __future__ import annotations

import argparse
import json
import sys

from .config import run_id


def cmd_bootstrap(args) -> None:
    from .bootstrap import asjp, curated, dplace, glottolog, iecor, wikidata, wikipedia

    mods = {
        "glottolog": glottolog,
        "wikidata": wikidata,
        "wikipedia": wikipedia,
        "iecor": iecor,
        "dplace": dplace,
        "asjp": asjp,
        "curated": curated,
    }
    names = list(mods) if args.which == "all" else [args.which]
    for n in names:
        print(f"== bootstrap {n}")
        print(json.dumps({k: v for k, v in mods[n].run().items() if k != "trees"}))


def cmd_synthesize(args) -> None:
    from .registry import publish

    # A family change can affect a shared evidence/decision digest: publish globally.
    print(json.dumps(publish(), indent=2))


def cmd_tasks(args) -> None:
    from .tasks import build_tasks

    tasks = build_tasks(refresh=args.refresh)
    for t in tasks[:25]:
        print(
            f"{t['rank']:3} {t['glottocode']} {t['name']:34} own langs={t['languoids']:4} own wikt={t['wiktionary_codes']:4} subtree lemmas={t['subtree_lemmas']:8}"
        )
    print(f"... {len(tasks)} tasks written")


def cmd_discover(args) -> None:
    from .discover import discover

    print(
        json.dumps(
            discover(
                args.family,
                max_results=args.max,
                screen=args.screen,
                batch=args.batch,
                walk=not args.no_walk,
                recommend=args.recommend,
            ),
            indent=1,
        )
    )


def cmd_retrieve(args) -> None:
    from .retrieve import retrieve_family

    print(json.dumps(retrieve_family(args.family), indent=1))


def cmd_extract(args) -> None:
    from .llm import extract_family

    print(
        json.dumps(
            extract_family(args.family, run=args.run or run_id(), batch=args.batch), indent=1
        )
    )


def cmd_batch_poll(args) -> None:
    from .batch import poll

    print(json.dumps(poll(args.family), indent=1))


def cmd_costs(args) -> None:
    """Sum model spend and tokens from every *_log.jsonl, by backend/model/role."""
    import glob, os
    from collections import defaultdict
    from .config import DATA_DIR

    tot = defaultdict(
        lambda: {
            "calls": 0,
            "cost_usd": 0.0,
            "prompt_tokens": 0,
            "completion_tokens": 0,
            "cached_tokens": 0,
        }
    )
    for path in glob.glob(os.path.join(DATA_DIR, "*", "*_log.jsonl")):
        role = os.path.basename(path).split("_")[0]
        with open(path, encoding="utf-8") as f:
            for line in f:
                r = json.loads(line)
                if "model" not in r:
                    continue
                u = r.get("usage") or {}
                k = f"{role:10} {r.get('backend', '?'):13} {r.get('model', '?')}"
                t = tot[k]
                t["calls"] += 1
                t["cost_usd"] += r.get("cost_usd") or 0
                t["prompt_tokens"] += u.get("prompt_tokens") or u.get("input_tokens") or 0
                t["completion_tokens"] += u.get("completion_tokens") or u.get("output_tokens") or 0
                t["cached_tokens"] += (
                    (u.get("prompt_tokens_details") or {}).get("cached_tokens")
                    or u.get("cached_input_tokens")
                    or u.get("cache_read_input_tokens")
                    or u.get("cache_read")
                    or 0
                )
    for k, t in sorted(tot.items()):
        print(
            f"{k:60} calls={t['calls']:5} cost=${t['cost_usd']:.4f} in={t['prompt_tokens']:9} out={t['completion_tokens']:8} cached={t['cached_tokens']:8}"
        )
    print(
        f"total reported cost ${sum(t['cost_usd'] for t in tot.values()):.4f} (claude-agent costs are nominal estimates; codex uses the ChatGPT subscription and does not report dollar cost)"
    )


def cmd_llm_check(args) -> None:
    from .llm import llm_check
    from .llm_backends import BACKENDS

    names = args.backends or list(BACKENDS)
    results = llm_check(names)
    print(json.dumps(results, indent=1))
    parsed = [json.dumps(r["parsed"], sort_keys=True) for r in results if r.get("ok")]
    print(
        "all backends agree"
        if len(set(parsed)) == 1 and len(parsed) == len(names)
        else f"{len(set(parsed))} distinct outputs across {len(parsed)} working backends"
    )


def cmd_llm_synthesize(args) -> None:
    from .llm import synthesize_family

    print(json.dumps(synthesize_family(args.family, run=args.run or run_id()), indent=1))


def cmd_validate(args) -> None:
    from .validate import validate_all

    errors = validate_all()
    for e in errors[:50]:
        print(e)
    print(f"{len(errors)} errors")
    if errors:
        sys.exit(1)


def cmd_export_periods(args) -> None:
    from .periods import write_periods

    print(json.dumps(write_periods(args.out), indent=2))


def cmd_report(args) -> None:
    from .registry import current_bundle, report

    print(report(current_bundle()))


def cmd_benchmark(args) -> None:
    from .benchmark import evaluate

    result = evaluate(args.predictions)
    print(json.dumps(result, indent=2))
    if not result["passed"]:
        sys.exit(1)


def cmd_repair_pilots(args) -> None:
    from .pilot_sources import repair

    print(json.dumps(repair(), indent=2))


def main() -> None:
    ap = argparse.ArgumentParser(
        prog="dating", description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    sub = ap.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("bootstrap")
    p.add_argument("which", nargs="?", default="all")
    p.set_defaults(fn=cmd_bootstrap)
    p = sub.add_parser("synthesize")
    p.add_argument("--family")
    p.add_argument("--run")
    p.set_defaults(fn=cmd_synthesize)
    p = sub.add_parser("tasks")
    p.add_argument("--refresh", action="store_true")
    p.set_defaults(fn=cmd_tasks)
    p = sub.add_parser("discover")
    p.add_argument("family")
    p.add_argument("--max", type=int, default=60)
    p.add_argument("--screen", choices=["llm", "keyword", "none"], default="llm")
    p.add_argument("--batch", action="store_true")
    p.add_argument("--no-walk", action="store_true", help="skip the S2 citation walk")
    p.add_argument(
        "--recommend",
        action="store_true",
        help="add an S2 recommendation round (includes as positives); low yield in pilots, so opt-in",
    )
    p.set_defaults(fn=cmd_discover)
    p = sub.add_parser("retrieve")
    p.add_argument("family")
    p.set_defaults(fn=cmd_retrieve)
    p = sub.add_parser("extract")
    p.add_argument("family")
    p.add_argument("--run")
    p.add_argument("--batch", action="store_true")
    p.set_defaults(fn=cmd_extract)
    p = sub.add_parser("batch-poll")
    p.add_argument("family")
    p.set_defaults(fn=cmd_batch_poll)
    p = sub.add_parser("costs")
    p.set_defaults(fn=cmd_costs)
    p = sub.add_parser("llm-check")
    p.add_argument("backends", nargs="*")
    p.set_defaults(fn=cmd_llm_check)
    p = sub.add_parser("llm-synthesize")
    p.add_argument("family")
    p.add_argument("--run")
    p.set_defaults(fn=cmd_llm_synthesize)
    p = sub.add_parser("validate")
    p.set_defaults(fn=cmd_validate)
    p = sub.add_parser("export-periods")
    p.add_argument("--out")
    p.set_defaults(fn=cmd_export_periods)
    p = sub.add_parser("report")
    p.set_defaults(fn=cmd_report)
    p = sub.add_parser("benchmark")
    p.add_argument("--predictions")
    p.set_defaults(fn=cmd_benchmark)
    p = sub.add_parser("repair-pilot-sources")
    p.set_defaults(fn=cmd_repair_pilots)
    for name, fn in (("import-evidence", "import_evidence"), ("decide", "decide")):
        p = sub.add_parser(name)
        p.add_argument("path")
        p.set_defaults(
            fn=lambda args, method=fn: print(
                json.dumps(
                    getattr(__import__("harness.review", fromlist=[method]), method)(args.path),
                    indent=2,
                )
            )
        )
    args = ap.parse_args()
    from .io_utils import locked
    from .config import DATA_DIR

    # One writer for legacy state across all CLI processes (includes batch ingestion).
    # Read-only inspection remains available; internal store operations also lock.
    if args.cmd in {
        "discover",
        "retrieve",
        "extract",
        "batch-poll",
        "bootstrap",
        "synthesize",
        "llm-synthesize",
        "repair-pilot-sources",
        "import-evidence",
        "decide",
    }:
        with locked(DATA_DIR + "/_pipeline"):
            args.fn(args)
    else:
        args.fn(args)


if __name__ == "__main__":
    main()
