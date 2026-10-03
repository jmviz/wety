# Uralic “hundred”: conditional contact case

The Permic forms do not, by themselves, decide between an earlier loan with regular development and separate borrowing. The [review packet](../../../data/workspaces/dating/_research/packets/uralic-hundred-2026-09-19/packet.json) retains eleven findings and a [structured case](../../../data/workspaces/dating/_research/packets/uralic-hundred-2026-09-19/case.json). Neither account receives a calendar date or a global registry relation.

## Sources and scope

The [Holopainen loan appendix](https://doi.org/10.5281/zenodo.6035900), column DW, labels the donor *ćata- under the literal row label “PII or ambiguous (PII ~PI)”. The workbook’s Iranian examples (CB4, EH4, EW4) support reading PI as Proto-Iranian, not Proto-Indic; the label does not select one donor stage for this loan. It gives recipient *ći̬ta under “PU, PFU,” evaluates the overall etymology positively, but marks Komi and Udmurt with question marks. DW12 offers parallel borrowing as a possibility and cites a regular sound-law alternative. These are different levels of judgment; an overall positive label must not erase branch uncertainty. Blank Samoyedic cells have not been converted into demonstrated absence.

The journal supplies [Zhivlov’s original PDF](https://www.jolr.ru/files/%28151%29jlr2014-12%28113-148%29.pdf). Appendix 2 (printed pp.124–125) gives a conditioned Permic vowel reflex, including in the environment preceding intervocalic *t. Appendix 3 (p.132) includes “hundred” in the relevant reconstruction class and lists Komi śo and Udmurt śu. The table, note 15 and lexical entry were visually inspected. This establishes what the proposed account says, not the independent adequacy of its correspondence set.

Two source-handling issues matter:

- The journal labels volume 12 as 2014, while the [publisher platform](https://doi.org/10.31826/jlr-2015-120109) labels the article 2015. Both dates survive in the source record; this is one study, not two confirmations.
- The PDF’s legacy fonts yield control characters or wrong glyphs in extracted phonetic strings. Original extraction bytes and exact spans are preserved, with visual review locators. The more differentiated reconstruction in Zhivlov has not been silently replaced with the appendix’s notation. Canonical machine-readable phonetic transcription remains pending.

The appendix derives from Holopainen’s dissertation and explicitly uses Zhivlov for this comment. The research graph records that scoped dependency. The dissertation and underlying UEW entry have not been independently inspected in this pass.

## What follows under each account

| Account | Conditional consequence | What remains open |
|---|---|---|
| Earlier borrowing, regular Permic development | If the borrowed form participated in this change, entry into the relevant recipient lineage precedes the change. This is an editorial logical inference, explicitly conditioned on participation. | The rule alone does not prove one common borrowing, a Proto-Uralic crown, recipient unity, locality or calendar age. Later substitution, analogy and diffusion require separate assessment. |
| Separate acquisitions | The case permits distinct Permic-related and other recipient loans. | The reviewed material supplies no order between them and no justified position relative to the vowel change. Do not manufacture the reverse of the other account’s edge. |

“Before *t” describes phonetic environment. It is not itself a temporal statement. These accounts are neither demonstrated exhaustive alternatives nor necessarily mutually exclusive across all recipient branches. Regular sound change and multiple contact events could coexist.

## Implementation and remaining work

`harness.contact_cases` validates source-finding references, scenario membership and cycles separately for each conditional graph. `harness.packets` binds the case hash to its source packet and prohibits a contact-case packet from simultaneously exporting unconditional claims or relations. This is a small editorial representation, not a new historical inference engine or downstream etymology solver.

Publication coverage inventories every reviewed packet, including sources outside the minimum pilot bibliography. Packet/case changes therefore participate in the generation's input digest. The root `pyproject.toml` and lockfile are also hashed; no removed package-specific environment is assumed.

Run from the repository root:

```sh
.venv/bin/python -m harness.packets data/workspaces/dating/_research/packets/uralic-hundred-2026-09-19/packet.json --require-imported
```

Next, review the correspondence set against counterexamples, verify source-specific transcriptions and donor adaptations, and have another reviewer assess the competing histories. The [annotation protocol](review-protocol.md) keeps those judgments separate from source fidelity. This case is exposed development material; it supplies neither specialist consensus nor held-out accuracy.
