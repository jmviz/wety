"""Offline checks of the LiLa converter: its reading rules and a small lemonEty graph."""

import json
import tempfile
import unittest
from pathlib import Path

from etymology import assertions as A
from etymology.convert import lila
from rdflib import Graph
from test_assertions import EXTRACTION

TTL = """
@prefix e: <http://lila-erc.eu/data/lexicalResources/BrillEDL/id/> .
@prefix ety: <http://lari-datasets.ilc.cnr.it/lemonEty#> .
@prefix ontolex: <http://www.w3.org/ns/lemon/ontolex#> .
@prefix lime: <http://www.w3.org/ns/lemon/lime#> .
@prefix lexinfo: <https://lexinfo.net/ontology/2.0/lexinfo.owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix b: <https://dictionaries.brillonline.com/search#dictionary=latin&id=> .

b:la1 a ontolex:LexicalEntry ; rdfs:label "legō" ;
    ety:etymology e:etymology\\/1 , e:etymology\\/2 .
b:la1\\#adv a ontolex:LexicalEntry ; rdfs:label "legō (adv.)" ; ety:cognate b:la1 .
e:etymology\\/1 a ety:Etymology ; ety:etymon e:etymon\\/pit1 , e:etymon\\/pie1 ;
    ety:hasEtyLink e:etylink\\/1 , e:etylink\\/2 ; ety:startingLink e:etylink\\/2 .
e:etymology\\/2 a ety:Etymology ; ety:etymon e:etymon\\/pie2 ;
    ety:hasEtyLink e:etylink\\/3 ; ety:startingLink e:etylink\\/3 .
e:etylink\\/1 a ety:EtyLink ; ety:etyLinkType "inheritance" ;
    ety:etySource e:etymon\\/pit1 ; ety:etyTarget b:la1 .
e:etylink\\/2 a ety:EtyLink ; ety:etyLinkType "inheritance" ;
    ety:etySource e:etymon\\/pie1 ; ety:etyTarget e:etymon\\/pit1 .
e:etylink\\/3 a ety:EtyLink ; ety:etyLinkType "inheritance" ;
    ety:etySource e:etymon\\/pie2 ; ety:etyTarget b:la1 .
e:etymon\\/pit1 a ety:Etymon ; rdfs:label "*leg‑e/o‑" ; lime:language "PIt" ;
    ontolex:canonicalForm [ ontolex:writtenRep "*leg‑e‑" , "*leg‑o‑" ] .
e:etymon\\/pie1 a ety:Etymon ; rdfs:label "pr. *leǵ‑e/o‑?" ; lime:language "PIE" ;
    rdfs:comment "pr. *leǵ‑e/o‑?" ;
    ontolex:canonicalForm [ ontolex:writtenRep "pr. *leǵ‑e/o‑?" ; lexinfo:tense lexinfo:present ] ;
    ontolex:otherForm [ ontolex:writtenRep "leǵ‑to‑" ; lexinfo:gender lexinfo:femine ] .
e:etymon\\/pie2 a ety:Etymon ; rdfs:label "*gʷʰ3‑eu‑m ;*x" ; lime:language "PIE" ;
    ontolex:canonicalForm [ ontolex:writtenRep "*a (tʰus LIV);*b" ] ;
    ety:cognate e:etymon\\/pie1 .
"""

READINGS = {
    "works": {"liv": {"citation": "LIV, as de Vaan cites it"}},
    "forms": [
        {
            "etymon": "etymon/pie2",
            "written": "*a (tʰus LIV);*b",
            "parts": [
                {"form": "*a", "credit": {"work": "liv", "role": "follows"}},
                {"form": "*b", "relation": "variant"},
            ],
            "decision": "test",
        }
    ],
    "comments": [],
}


class Read(unittest.TestCase):
    def test_hedge_label_and_missing_asterisk(self):
        self.assertEqual(
            lila.read("pr. *leǵ‑e/o‑?")[0], {"form": "*leǵ‑e/o‑", "label": "pr.", "hedge": "?"}
        )
        self.assertEqual(lila.read("*sm‑ [in cp.]")[0], {"form": "*sm‑", "label": "in cp."})
        self.assertEqual(lila.read("klh₁‑")[0], {"form": "*klh₁‑"})

    def test_conversion_damage(self):
        self.assertEqual(lila.read("*gʷʰ3‑eu‑m")[0]["form"], "*gʷh₃‑eu‑m")
        self.assertEqual(lila.read("*koχο‑")[0]["form"], "*koχo‑")
        self.assertEqual(lila.read("*bʰleh₃‑ōs >")[0]["form"], "*bʰleh₃‑ōs")

    def test_several_forms_are_not_read(self):
        self.assertIsNone(lila.read("*de;*do")[0])
        self.assertIsNone(lila.read("*X‑þo‑ >> *X‑þāsjo‑")[0])

    def test_unknown_label_words_are_errors(self):
        with self.assertRaises(ValueError):
            lila.read("zz. *a")


class Convert(unittest.TestCase):
    def setUp(self):
        g = Graph()
        g.parse(data=TTL, format="turtle")
        with tempfile.TemporaryDirectory() as d:
            path = Path(d) / "forms.json"
            path.write_text(json.dumps(READINGS))
            self.c = lila.Converter(g, EXTRACTION, lila.Readings(path))
        self.c.all()
        self.objs = list(self.c.out.objects.values())
        self.by_id = {o["id"]: o for o in self.objs}

    def test_valid_and_complete(self):
        self.assertEqual(A.validate(self.objs), [])
        types = [o["type"] for o in self.objs]
        self.assertEqual(types.count("hypothesis"), 2)
        self.assertEqual(types.count("record"), 5)

    def test_two_etymologies_are_exclusive_alternatives(self):
        alt = next(o for o in self.objs if o["type"] == "alternative_set")
        self.assertTrue(alt["exclusive"])
        self.assertIsNone(alt["preference"])
        members = [o for o in self.objs if o["type"] == "assertion" and "in_sets" in o]
        self.assertEqual(len(members), 2)

    def test_record_forms_hedges_and_features(self):
        pit = self.by_id["lila-edl:etymon/pit1"]
        self.assertEqual(pit["form"], "*leg‑e/o‑")
        pie = self.by_id["lila-edl:etymon/pie1"]
        self.assertEqual(pie["form"], "*leǵ‑e/o‑")
        self.assertEqual(pie["certainty"], {"level": "possible", "words": "?"})
        other = pie["x"]["lila-edl"]["forms"][1]
        self.assertEqual((other["form"], other["features"]), ("*leǵ‑to‑", ["gender=feminine"]))

    def test_credits_and_cognates(self):
        a = next(o for o in self.objs if o["type"] == "assertion" and o["locator"] == "etymology/2")
        self.assertEqual(
            a["credits"], [{"work": "work:lila-edl:liv", "role": "follows", "scope": "inline"}]
        )
        adv = self.by_id["lila-edl:la1#adv"]
        self.assertEqual((adv["form"], adv["homograph"]), ("legō", "adv."))
        self.assertEqual(adv["x"]["lila-edl"]["cognates"], ["lila-edl:la1"])


if __name__ == "__main__":
    unittest.main()
