import collections
import unittest

from etymology.convert import etymdict


class Newick(unittest.TestCase):
    def test_comments_and_unnamed_nodes(self):
        parents, root = etymdict.newick("(((a,b)[Ab linkage],(c)pc[C group])poc)early;")
        self.assertEqual(root, "early")
        self.assertEqual(parents["a"], "_0")
        self.assertEqual(parents["_0"], "poc")
        self.assertEqual(parents["c"], "pc")
        self.assertEqual(parents["poc"], "early")


class PlacedTree(unittest.TestCase):
    def test_a_language_hangs_from_a_placed_proto_language(self):
        parents, root = etymdict.newick("((x)poc)pemp")
        place = {"pcb": "pemp", "ambai": "pcb"}
        tree = etymdict.Tree(parents, root, {"poc", "pemp", "pcb"}, place)
        self.assertEqual(tree.above("ambai"), ["pcb", "pemp"])
        self.assertEqual(tree.above("x"), ["poc", "pemp"])
        self.assertEqual(tree.above("unplaced"), [])


def release(rows, place):
    """A release of one set (`rows`: form id, language, the first is its reconstruction)
    on the tree pan > poc > pt, with attested languages placed by `place`."""
    r = object.__new__(etymdict.Release)
    protos = {"pan", "poc", "pt"}
    r.languages = {lang: {"Is_Proto": "true" if lang in protos else "false"} for _, lang in rows}
    r.forms = {f: {"Language_ID": lang, "Loan": "", "Doubt": "false"} for f, lang in rows}
    r.sets = {"s": {"Form_ID": rows[0][0], "Doubt": "false"}}
    r.cognates = {
        "s": [{"ID": f"c{i}", "Form_ID": f, "Doubt": "false"} for i, (f, _) in enumerate(rows)]
    }
    parents, root = etymdict.newick("((pt)poc)pan")
    r.tree = etymdict.Tree(parents, root, protos, place)
    return r


class Compiler(etymdict.Converter):
    NAME = "test"

    def __init__(self, r, first=True):
        self.r, self.RECONSTRUCTION_FIRST = r, first
        self.counts = collections.Counter()
        self.links = {}

    def compiled_link(self, what, link, locator, credit_locator, hedges=()):
        self.links[link["child"].split(":")[1]] = link["parents"][0].split(":")[1]


class NestedRule(unittest.TestCase):
    def test_reflexes_follow_the_reconstruction_they_list_after(self):
        rows = [("pan1", "pan"), ("poc1", "poc"), ("a", "motu"), ("poc2", "poc"), ("b", "motu")]
        c = Compiler(release(rows, {"motu": "pt"}))
        c.compile()
        self.assertEqual(c.links, {"poc1": "pan1", "a": "poc1", "b": "poc2"})
        self.assertEqual(c.counts["a second reconstruction at a level its set has: no link"], 1)

    def test_reflexes_before_their_subgroups_reconstruction(self):
        rows = [("poc1", "poc"), ("a", "motu"), ("pt1", "pt"), ("b", "fiji")]
        c = Compiler(release(rows, {"motu": "pt", "fiji": "poc"}), first=False)
        c.compile()
        self.assertEqual(c.links, {"a": "pt1", "pt1": "poc1", "b": "poc1"})
        self.assertEqual(
            c.counts["reconstructions with no form of their set above them: no link"], 1
        )

    def test_a_form_with_nothing_of_its_set_above_it(self):
        rows = [("pt1", "pt"), ("poc1", "poc"), ("x", "outside")]
        c = Compiler(release(rows, {}))
        c.compile()
        self.assertEqual(c.links, {"pt1": "poc1"})
        self.assertEqual(c.counts["members with no form of their set above them: no link"], 2)


if __name__ == "__main__":
    unittest.main()
