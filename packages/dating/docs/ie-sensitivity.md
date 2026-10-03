# IE sensitivity deposit check

20 September 2026; development self-audit. The complete 161,771,042-byte Light ZIP from [Zenodo 8147476](https://zenodo.org/records/8147476) is retained through the shared acquirer. SHA-256 is `ec6cdfc18b9e3ae96599e71ce2bd81588e397d6921a2db04ff6c2c404a9d3482`; MD5 matches the deposit's `d8b8ba1fdca572bcd40fde8465e4d908`. Its original bytes, selected XML/log/MCC members, acquisition pointer, audit and provenance are retained. No production claims or defaults are added.

Run `.venv/bin/python -m harness.ie_sensitivity_checks` from the repository root. It validates the archive hash before reading named members. It retains native heights and an explicit factor-1000 comparison with the published year-BP summaries; it does not assign an epoch or authorize production normalization. All 27,003 rows of each author-combined log are used. The guide §§4.2/5.4 describes combination, three independent initial chains and their burn-in; applying another generic 10% to the combined logs would be unjustified.

| Scenario | Taxa in XML alignment | Explicit ancestry constraints | Combined median native height | Comparison in years BP |
| --- | ---: | ---: | ---: | ---: |
| SA7a Latin only | 161 | 1 | 7.788517082879079 | 7,788.517 |
| SA8a Chang broad, ancestry enabled | 94 | 0 | 8.629179873141495 | 8,629.180 |
| SA8b Chang broad, ancestry constrained | 94 | 8 | 4.996552741145007 | 4,996.553 |

SA7a rounds to 7,789: this agrees with the article's stated 331-year reduction from 8,120 and 4.08%, but conflicts with its printed endpoint 7,889. Preserve both the article assertion and deposited result; do not silently correct a quote. This resolves what the deposited log reports, not the origin of the publication error.

SA8a and SA8b reproduce the reported rounded medians and have identical hashed taxon/sequence alignments. Their XML explicitly contains zero versus eight ancestry constraints. This supports a controlled comparison within Heggarty's deposited analyses, not equivalence to Chang's complete original inference. Other configuration differences, chain-level mixing, software/runtime and prior sensitivity still need assessment before attributing the entire numerical effect to one isolated intervention.

Both SA8 XMLs contain 43 distributions whose IDs start `monophyly(` (44 non-tip monophyletic MRCA distributions altogether). The guide describes 35 clade constraints. This is a configuration-count reconciliation question: raw element counts are not necessarily counts of distinct nonredundant historical constraints. The audit preserves the 43 identifiers; it does not declare the guide wrong or infer 43 independent constraints.

The Light deposit lacks the full posterior-tree and individual-chain material required for stronger diagnostics. Combined traces alone do not verify independent-chain convergence, and MCC trees do not preserve joint topology uncertainty. The main supplementary methods and Chang's original files remain access/recovery work. Existing packets and claim identities are unchanged.

Harness implications: XML/log files are methodological evidence; do not discard them as “dateless.” Enumerate files blocked by acquisition limits (the Light ZIP exceeds the current automatic cap), route them to explicit recovery and specialized parsers, compare reported versus computed quantities, preserve discrepancies, and record which diagnostics were actually checked. These are concrete requirements for future autonomous family runs.

The editorial file set is preserved as `dating-ie-sensitivity-review/31087201b696c9a6596716449d4a6a07ce373cabdbc8ec63ec8d6c0f4696f3d5`. This adoption records a reproducible review artifact, not scientific approval.
