# Status

*Updated 2026-10-03. Branch `dating-pilot` (from `reorg-baseline`), pushed; not merged.*

## What the project promises

wety shows Wiktionary etymologies as trees. Behind a toggle, language nodes show when
the language was spoken: an interval, its basis and citations, from reviewed evidence
in the `packages/dating` registry.

## What is usable today

- **In the app:** a "dates" toggle on the view bar. **11 of the 20 pilot codes gained a
  displayable, cited period.** Eight are full intervals: `ang enm gmh` (ISO 639-3
  conventional periods), `hit och ltc` (Glottolog documentation windows, shown as
  attestation), and `gem-pro cel-pro` (Koch 2020 estimates). Three are partial:
  `sga` and `grc` (end only), `la` (first written record only). The node popover
  shows quote, locator, link and other sources' periods. Checked in a browser on the
  legacy graph (chart and outline).
- `GET /periods` serves `data/products/dating/language_periods.json` (tracked;
  `WETY_PERIODS_PATH` overrides; a missing file serves an empty list).
- Registry: 3,614 claims (22 supported), 11 adopted decisions, 0 family splits (none
  supported).

## What changed

- `prepare --brief FILE`; pilot brief in `packages/dating/prompts/period-verification.md`.
- One Astra run (gpt-6-astra, high effort, Codex on ChatGPT): 1,017 s of 3,300 allowed;
  2.09M input tokens (1.95M cached), 22k output. It used all 30 fetches and 250 of
  260 tool actions (183 reads, mostly paging the Glottolog CSV). Of the 30 existing
  candidate claims, it confirmed 2, corrected 22 and found 6 unverifiable.
- My review corrected 6 items: two mappings set to exact (`enm`, `ltc`), two
  "to N" ends re-encoded as open-start periods, one secondary report demoted, and an
  implausible Akkadian value demoted. Details: [run README](data/workspaces/dating/_research/period-pilot-2026-10-03/README.md).
- `harness.cli export-periods`; registry support for separately stated endpoints
  (mixed eras, uncertain bounds); fixed CSV mirror validation.

## Uncertainties

- Most adopted sources are catalogs (ISO, Glottolog), not handbooks. The four
  handbooks behind earlier claims were unreadable; their old "quotes" were from memory.
- Glottolog windows are documentation metadata. Gothic's (350–380) was too narrow to
  show.
- No usable period: `goh non got cu akk sa ojp ine-pro sla-pro`. The causes are scope
  mismatches (Wiktionary files later stages under `cu`, `sa`, `ojp`), missing sources,
  or estimates not mapped to the Wiktionary entity (`ine-pro`).
- Two model sessions reviewed; not independent corroboration.
- The Codex CLI is not on PATH: the run used ChatGPT.app's bundled copy.

## Next three actions

1. Give Astra a handbook-access route (library PDFs supplied locally) so conventional
   periods rest on scholarly sources. Retry the 9 missing codes.
2. Add Glottolog-style table lookup to the research tools, so one table row no longer
   costs dozens of reads.
3. Retention: `gc --dry-run` keeping the current generation, the previous two, and
   anything referenced.

## Decisions needed from the maintainer

- Whether ISO 639-3 names and Glottolog documentation windows are acceptable display
  sources, or only as alternatives.
- The two reviewer mapping overrides (`enm`, `ltc`).
- When to merge `dating-pilot` into `main` (it deploys); the host needs the new server
  build.
- Whether to narrow the dating fingerprint: every harness edit invalidates decisions.
