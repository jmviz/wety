## TODO

### Potential improvements to item disambiguation / ety link inference

* Use senseid's to augment disambiguation. Seem to be fairly common in e.g. Middle English, which has tons of ambiguous terms. 

## Things to keep in mind

* For many languages, Wiktionary makes a distinction between a term's "entryName" (the written form that is the title of its page) and the forms used in etymology sections: Latin page titles omit macrons, which etymology template arguments include. `Language:makeEntryName` in [Module:languages](https://en.wiktionary.org/wiki/Module:languages) makes the conversion, by rules per language. The processor takes entry names from the links the rendered pages make (`wiktionary_entries`' `entry_names.tsv.gz`); a term no page renders as a link has none, and is looked up as written and through redirects.

* If ever out of immediate TODOs in this document, remember to ctrl+f "$" in project to find notes pointing out problems/todos.

### Processing more of the etymology

* Some etymologies on Wiktionary (e.g. https://en.wiktionary.org/wiki/astrology) have {{der}}-type chains followed by a template in this category which recapitulates the etymology through surface analysis. Simply treating all templates the same and chugging through the chain will result in a lot of bad ety connections. A simple provisional solution might be to only take the first compound-type template (with "1" parameter being the language of the item term), if one is present, discarding everything else. This will lose the actual historical etymology information if there is any (i.e. the derived-type chain), but might lead to most reliably far-reaching derivation chains.
