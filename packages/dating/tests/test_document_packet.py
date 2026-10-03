import tempfile
from pathlib import Path
import unittest
from harness.document_packet import parse_article

class PacketTests(unittest.TestCase):
    def test_body_is_separate_from_navigation_and_attachments_are_explicit(self):
        with tempfile.TemporaryDirectory() as tmp:
            p=Path(tmp)/'article.html'
            p.write_text('<nav>not source</nav><article><section id="methods"><h2>Methods</h2><p>'+('Native text. '*200)+'</p><script>ignored</script><a href="extra.xlsx">Data</a><img src="figure.png" alt="a plot"></section></article><footer>not article</footer>')
            text,meta=parse_article(p,'https://example.org/article/')
            self.assertNotIn('not source',text);self.assertNotIn('ignored',text);self.assertNotIn('not article',text)
            self.assertEqual(meta['sections'],['methods'])
            self.assertEqual(meta['attachments'][0]['url'],'https://example.org/article/extra.xlsx')
            self.assertEqual(meta['images'][0]['status'],'not_supplied_as_pixels')
            heading=meta['headings'][0]
            self.assertEqual(text[heading['start']:heading['end']],'Methods')
    def test_landing_page_is_not_full_article(self):
        with tempfile.TemporaryDirectory() as tmp:
            p=Path(tmp)/'landing.html';p.write_text('<p>'+('large navigation '*500)+'</p>')
            with self.assertRaises(ValueError):parse_article(p,'https://example.org')

if __name__=='__main__':unittest.main()
