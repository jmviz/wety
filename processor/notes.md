# ai

## fine-tuning and distillation

- https://github.com/unslothai/unsloth
  - phi-4: https://colab.research.google.com/github/unslothai/notebooks/blob/main/nb/Phi_4-Conversational.ipynb

## "production" model considerations

### models

- https://huggingface.co/Qwen/QwQ-32B

### system prompt

- https://github.com/x1xhlol/v0-system-prompts/blob/main/v0.txt

### serving

Dedicated GPU(s) combined with a serving engine will be best:

- https://docs.vllm.ai/en/latest/index.html
- https://docs.sglang.ai/index.html

These all support structured output as well as prefix caching and continuous batching, which will greatly speed up this use case.

### hosting 

- https://vast.ai/
- https://www.runpod.io/

## models for prototyping

### local (<=10 GB VRAM)

- https://huggingface.co/microsoft/Phi-4-mini-instruct (?)
- https://huggingface.co/ibm-granite/granite-3.2-2b-instruct (?)
- https://huggingface.co/Qwen/Qwen2.5-7B-Instruct-GPTQ-Int8 (cf. https://qwen.readthedocs.io/en/latest/benchmark/speed_benchmark.html which gives VRAM, tokens/s, VLLM settings)
-

### hf serverless

- https://huggingface.co/google/gemma-2-27b-it

### hf api

- https://huggingface.co/models?inference=warm&sort=trending

## task kinds

### structured generation

- https://docs.sglang.ai/backend/structured_outputs.html
- https://docs.vllm.ai/en/latest/features/structured_outputs.html
- https://huggingface.co/learn/cookbook/en/structured_generation
- https://cookbook.openai.com/examples/structured_outputs_intro

### rag

- https://huggingface.co/docs/smolagents/examples/rag (tool use as well)
- https://github.com/wikimedia-enterprise/Structured-Contents-LLM-RAG
- https://github.com/tylertitsworth/multi-mediawiki-rag

### embedding

- https://huggingface.co/spaces/mteb/leaderboard

# fonts

## woff2

Convert ttf to woff2 to compress and thus minimize file download size for users when declaring font-face. https://github.com/google/woff2:

```bash
brew install woff2
find public/fonts -name "*.ttf" -exec woff2_compress {} \;
```

## wiktionary html lang and script -> font

When parsing html, get wiktionary script type class(es) and html lang attribute for all terms. Can e.g. be used to provide best fonts per script, as wiktionary does. E.g. for [ῠ̔́δρᾱ • (hŭ́drā)](https://en.wiktionary.org/wiki/%E1%BD%95%CE%B4%CF%81%CE%B1#Ancient_Greek), there is html:

```html
<strong class="Polyt headword" lang="grc">ῠ̔́δρᾱ</strong>
...
<span lang="grc-Latn" class="headword-tr tr Latn" dir="ltr">hŭ́drā</span>
```

where `Polyt` and `Latn` are Wiktionary-brewed script-type classes (for polytonic and Latin script, respectively), and `grc` and `grc-Latn` are [language tags](https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/lang#language_tag_syntax) for Greek and Greek written in Latin script, respectively.

Wiktionary then has many css rules like the following:

```css
.Polyt {
  font-family: "SBL Greek", "New Athena Unicode", "DejaVu Sans", Athena,
    Gentium, "Gentium Plus", "Palatino Linotype", Times, "Arial Unicode MS",
    "Lucida Sans Unicode", "Lucida Grande", "Code2000", sans-serif;
}
```

# Old

## TODO

### Potential improvements to item disambiguation / ety link inference

- Use senseid's to augment disambiguation. Seem to be fairly common in e.g. Middle English, which has tons of ambiguous terms.

## Things to consider for reducing RAM usage

## Things to keep in mind

- For many languages, Wiktionary makes a distinction between a term's "entryName" (i.e., the written form that acts as the title of its corresponding page, which is captured in wiktextract's `word` field.) and any "canonical" or otherwise used forms of the term as they may appear in etymology sections. For example, in Latin, macrons are omitted for a term's entryName, while macrons are supposed to be included in etymology sections (and therefore, in arguments in etymology templates). This leads to problems when trying to link a form listed in some etymology template argument with a corresponding wiktextract `word`. The rules for transforming a form of a term to its entryName form are contained in [Module:languages](https://en.wiktionary.org/wiki/Module:languages). In particular, the `Language:makeEntryName` function makes this conversion by applying specific rules for each language provided in the module's data submodules. For many languages, the rules are fairly simple character substitutions; but for others, the rules are a module unto themselves. Therefore it would not be completely trivial to recreate all this behavior de novo.

  - Currently, this is addressed by preferring the wiktextract "canonical" form (when it exists in the json) over the `word` as the term for each item on which we hash. N.B. the "canonical" form is often listed in wiktextract entries in `forms[0].form` when `forms[0].tags[0]` == `canonical` (However, the canonical form is not guaranteed to be the first listed).
    - If this proves problematic somehow, an alternative approach is to run the lua code directly from within the rust program to generate `entryName`s for all terms listed in etymology templates, so they can be appropriately linked with their corresponding entries.

- Whenever `inh+`/`der+`/`bor+`/`com+` (there may be others) appear in wikitext, wiktextract inserts two imputed ety templates into the list of templates before listing the `+` template, see e.g. https://kaikki.org/dictionary/All%20languages%20combined/meaning/%D1%80/%D1%80%D0%B8/%D1%80%D0%B8%D1%81%D0%BE%D0%B2%D0%B0%D1%82%D1%8C.html. Therefore if the first actual ety template is a `+`, it will be third in the wiktextract template list. The first is of the form e.g.

```
{"name": "glossary", "args": {"1": "Inherited"}, "expansion": "Inherited"}
```

while the second is a non-`+` version of the template. Therefore, these imputatations shouldn't matter for us, as we currently take the first ety template that is on our lists in `etymology_templates.rs`. Therefore we will take the imputed second template, the non-`+` version, which has all the same info as the `+` version. However, if in the future we decide to try processing all the ety templates, these imputations will become relevant.

- If ever out of immediate TODOs in this document, remember to ctrl+f "$" in project to find notes pointing out problems/todos.

### If in the future I attempt to process the entire etymology (i.e. all templates and text)

- Need to handle when ety entries have multiple ety's for some reason, e.g. https://en.wiktionary.org/wiki/hap#Etymology_1. A simple approach might be to only process the first paragraph, as often the different ety's are listed in different paragraphs. Unfortunately, the templates given in wiktextract data are not separated out into paragraphs, so this would involve processing a combination of the wiktextract ety text (which does preserve the newlines) and the templates list. This can be done by using the expansions given for each template and looking for them in the ety text. In case a template expansion appears multiple times in the ety text in different paragraphs, can compare the order of the templates in the template list with the order of expansions in the ety text to try to infer which paragraph the template appeared in.
- When processing the ety templates, deal with case where there is a valid chain of derivs but there is a term amid it that doesn't have an item entry, while a subsequent term in the chain does.
- Some etymologies on Wiktionary (e.g. https://en.wiktionary.org/wiki/astrology) have {{der}}-type chains followed by a template in this category which recapitulates the etymology through surface analysis. Simply treating all templates the same and chugging through the chain will result in a lot of bad ety connections. A simple provisional solution might be to only take the first compound-type template (with "1" parameter being the language of the item term), if one is present, discarding everything else. This will lose the actual historical etymology information if there is any (i.e. the derived-type chain), but might lead to most reliably far-reaching derivation chains.
