"""Offline checks of the cited-term collection in source_tools.entry_names."""
import unittest
from source_tools.entry_names import clean, descendant_pairs, template_pairs

CODES = {'en', 'enm', 'fro', 'la', 'xno', 'grc', 'pt', 'it', 'ine-pro', 'gem-pro', 'gem'}


def pairs(name, args):
    return set(template_pairs({'name': name, 'args': args}, CODES))


class EntryNames(unittest.TestCase):
    def test_clean_matches_processor(self):
        # crates/processor clean_template_term
        self.assertEqual(clean('*gaberaną'), 'gaberaną')
        self.assertEqual(clean('*bʰel- (shiny)'), 'bʰel-')
        self.assertEqual(clean('twig#Etymology_2'), 'twig')
        self.assertEqual(clean('līber<id:free>'), 'līber')
        self.assertEqual(clean('[[lītus|lītoris]]'), 'lītus')

    def test_templates(self):
        self.assertIn(('la', 'mātrīx'), pairs('bor', {'1': 'en', '2': 'la', '3': 'mātrīx'}))
        self.assertIn(('ine-pro', 'bʰer-'), pairs('root', {'1': 'en', '2': 'ine-pro', '3': '*bʰer-'}))
        self.assertIn(('gem-pro', 'fōts'), pairs('word', {'1': 'en', '2': 'gem', '3': '*fōts'}))
        af = pairs('af', {'1': 'pt', '2': 'a-<id:privative>', '3': 'grc:χρῶμα<t:color>', '4': '-ia'})
        self.assertTrue({('pt', 'a-'), ('grc', 'χρῶμα'), ('pt', '-ia')} <= af)
        self.assertIn(('la', 'vexillum'), pairs('af', {'1': 'en', '2': 'vexillum', '3': '-ology', 'lang1': 'la'}))

    def test_etymon(self):
        got = pairs('etymon', {'1': 'en', '2': ':der', '3': 'xno:mot<ety:inh<fro:mot<ety:inh<la:muttum>>>>'})
        self.assertTrue({('xno', 'mot'), ('fro', 'mot'), ('la', 'muttum')} <= got)
        self.assertNotIn(('en', ':der'), got)

    def test_descendants(self):
        nodes = [{'lang_code': 'enm', 'word': 'moder', 'descendants': [{'lang_code': 'en', 'word': 'mother'}]}]
        self.assertEqual(set(descendant_pairs(nodes, CODES)), {('enm', 'moder'), ('en', 'mother')})


if __name__ == '__main__':
    unittest.main()
