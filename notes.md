## Immediate TODOs

* Add senseid to items. Parse in JSON by finding ':' then taking everything after.

* Implement better sense disambiguation.
    * Take PoSs into account. This will be particularly helpful when items have no gloss. 
    * Could add specific rules for the few templates where you would expect a different PoS, e.g. `deverbal`.
    * Could do better than simple Lesk algorithm. For example, "poison" and "poisoned" don't match but should count as similar. 

* Collect glosses from ety templates. These are quite commonly used and would be helpful for sense disambiguation as well as for having glosses for imputed terms. 

## Things to keep in mind

* For many languages, Wiktionary makes a distinction between a term's "entryName" (i.e., the written form that acts as the title of its corresponding page, which is captured in wiktextract's `word` field.) and any "canonical" or otherwise used forms of the term as they may appear in etymology sections. For example, in Latin, macrons are omitted for a term's entryName, while macrons are supposed to be included in etymology sections (and therefore, in arguments in etymology templates). This leads to problems when trying to link a form listed in some etymology template argument with a corresponding wiktextract `word`. The rules for transforming a form of a term to its entryName form are contained in [Module:languages](https://en.wiktionary.org/wiki/Module:languages). In particular, the `Language:makeEntryName` function makes this conversion by applying specific rules for each language provided in the module's data submodules. For many languages, the rules are fairly simple character substitutions; but for others, the rules are a module unto themselves.  Therefore it would not be completely trivial to recreate all this behavior de novo. 
    * Currently, this is addressed by preferring the wiktextract "canonical" form (when it exists in the json) over the `word` as the term for each item on which we hash. N.B. the "canonical" form is often listed in wiktextract entries in `forms[0].form` when `forms[0].tags[0]` == `canonical` (However, the canonical form is not guaranteed to be the first listed).
        * If this proves problematic somehow, an alternative approach is to run the lua code directly from within the rust program to generate `entryName`s for all terms listed in etymology templates, so they can be appropriately linked with their corresponding entries.

* Whenever `inh+`/`der+`/`bor+`/`com+` (there may be others) appear in wikitext, wiktextract inserts two imputed ety templates into the list of templates before listing the `+` template, see e.g. https://kaikki.org/dictionary/All%20languages%20combined/meaning/%D1%80/%D1%80%D0%B8/%D1%80%D0%B8%D1%81%D0%BE%D0%B2%D0%B0%D1%82%D1%8C.html. Therefore if the first actual ety template is a `+`, it will be third in the wiktextract template list. The first is of the form e.g. 
```
{"name": "glossary", "args": {"1": "Inherited"}, "expansion": "Inherited"}
```
while the second is a non-`+` version of the template. Therefore, these imputatations  shouldn't matter for us, as we currently take the first ety template that is on our lists in `etymology_templates.rs`. Therefore we will take the imputed second template, the non-`+` version, which has all the same info as the `+` version.  However, if in the future we decide to try processing all the ety templates, these imputations will become relevant. 

* If ever out of immediate TODOs in this document, remember to ctrl+f "$" in project to find notes pointing out problems/todos.

### If in the future I attempt to process the entire etymology (i.e. all templates and text)
* Need to handle when ety entries have multiple ety's for some reason, e.g. https://en.wiktionary.org/wiki/hap#Etymology_1. A simple approach might be to only process the first paragraph, as often the different ety's are listed in different paragraphs. Unfortunately, the templates given in wiktextract data are not separated out into paragraphs, so this would involve processing a combination of the wiktextract ety text (which does preserve the newlines) and the templates list. This can be done by using the expansions given for each template and looking for them in the ety text. In case a template expansion appears multiple times in the ety text in different paragraphs, can compare the order of the templates in the template list with the order of expansions in the ety text to try to infer which paragraph the template appeared in.
* When processing the ety templates, deal with case where there is a valid chain of derivs but there is a term amid it that doesn't have an item entry, while a subsequent term in the chain does.
* Some etymologies on Wiktionary (e.g. https://en.wiktionary.org/wiki/astrology) have {{der}}-type chains followed by a template in this category which recapitulates the etymology through surface analysis. Simply treating all templates the same and chugging through the chain will result in a lot of bad ety connections. A simple provisional solution might be to only take the first compound-type template (with "1" parameter being the language of the item term), if one is present, discarding everything else. This will lose the actual historical etymology information if there is any (i.e. the derived-type chain), but might lead to most reliably far-reaching derivation chains.