# Test case descriptions
Each file contains one or more entries as laid out below.

## glow.json.gz
English `glow` is both noun and verb. They both follow the same inheritance chain. This case features handling differences in `entryName` vs. `canonical form` as well as redirects.
### [wiktionary](https://en.wiktionary.org/wiki/glow)

```From {{inh|en|enm|glowen}}, from {{inh|en|ang|glōwan}}, from {{inh|en|gem-pro|*glōaną}}, from {{der|en|ine-pro|*ǵʰel-}}```

Also, `ǵʰel-` to `ǵʰelh₃-` [redirect](https://en.wiktionary.org/w/index.php?title=Reconstruction:Proto-Indo-European/%C7%B5%CA%B0el-&redirect=no).

### wiktextract

[glow](https://kaikki.org/dictionary/English/meaning/g/gl/glow.html), [glowen](https://kaikki.org/dictionary/All%20languages%20combined/meaning/g/gl/glowen.html), [glowan](https://kaikki.org/dictionary/All%20languages%20combined/meaning/g/gl/glowan.html), [glōaną](https://kaikki.org/dictionary/All%20languages%20combined/meaning/g/gl/gl%C5%8Dan%C4%85.html), [ǵʰelh₃-](https://kaikki.org/dictionary/All%20languages%20combined/meaning/%C7%B5/%C7%B5%CA%B0/%C7%B5%CA%B0elh%E2%82%83-.html)

Also, the redirect:

```
{"title": "Reconstruction:Proto-Indo-European/ǵʰel-", "redirect": "Reconstruction:Proto-Indo-European/ǵʰelh₃-"}
```

## fix.json.gz 
Cases testing prefix/suffix/circumfix/infix/confix handling. 
* [Prefix](https://en.wiktionary.org/wiki/Template:prefix) e.g. [redo](https://en.wiktionary.org/wiki/redo): `{{prefix|en|re|do}}` -> `re-` + `do`
* [Suffix](https://en.wiktionary.org/wiki/Template:suffix) e.g. [giftig](https://en.wiktionary.org/wiki/giftig#Dutch): `{{suffix|nl|gift|ig}}` -> `gift` + `-ig`
* [Circumfix](https://en.wiktionary.org/wiki/Template:circumfix) e.g. [vergiftigen](https://en.wiktionary.org/wiki/vergiftigen): `{{circumfix|nl|ver|giftig|en}}` -> `giftig` + `ver- -en`. Note `ver- -en` is one circumfix term, not two separate terms.
* [Infix](https://en.wiktionary.org/wiki/Template:infix) e.g. [hizouse](https://en.wiktionary.org/wiki/hizouse): `{{infix|en|house|iz}}` -> `house` + `-iz-`
* [Confix](https://en.wiktionary.org/wiki/Template:confix) e.g. [neurogenic](https://en.wiktionary.org/wiki/neurogenic): `{{confix|en|neuro|genic}}` -> `neuro-` + `-genic`; e.g. [bedewed](https://en.wiktionary.org/wiki/bedewed): `{{confix|en|be|dew|ed}}` -> `be-` + `dew` + `-ed`. That is, the first positional term arg is treated as a prefix, and the last positional term arg is treated as a suffix. There is an optional non-affix middle term.

## TODO senses.json.gz
Cases testing sense disambiguation. 
* en lime
* en see
* en bank