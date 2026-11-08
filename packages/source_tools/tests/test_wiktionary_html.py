import bz2
import tempfile
import unittest
from pathlib import Path

from source_tools.wiktionary_html import dump_pages

DUMP = """<mediawiki>
  <page>
    <title>Reconstruction:Proto-Germanic/frijaz</title>
    <ns>118</ns>
    <id>90</id>
    <revision>
      <id>92299146</id>
      <parentid>1</parentid>
      <contributor>
        <username>Someone</username>
        <id>77</id>
      </contributor>
      <text bytes="40" xml:space="preserve">==Proto-Germanic==
{{etymon|gem-pro|id=free|:inh|ine-pro:*priHós}} &amp; more</text>
    </revision>
  </page>
  <page>
    <title>Reconstruction:Proto-Germanic/frijô</title>
    <ns>118</ns>
    <id>91</id>
    <redirect title="Reconstruction:Proto-Germanic/frijaz" />
    <revision>
      <id>5</id>
      <text bytes="10" xml:space="preserve">#REDIRECT [[x]]</text>
    </revision>
  </page>
  <page>
    <title>free</title>
    <ns>0</ns>
    <id>92</id>
    <revision>
      <id>6</id>
      <text bytes="4" xml:space="preserve">text</text>
    </revision>
  </page>
  <page>
    <title>Reconstruction:Proto-Germanic/empty</title>
    <ns>118</ns>
    <id>93</id>
    <revision>
      <id>7</id>
      <text bytes="0" xml:space="preserve" />
    </revision>
  </page>
</mediawiki>
"""


class DumpPages(unittest.TestCase):
    def test_non_redirect_pages_of_the_namespace(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "dump.xml.bz2"
            path.write_bytes(bz2.compress(DUMP.encode()))
            pages = list(dump_pages(path, 118))
        self.assertEqual(
            pages,
            [
                (
                    "Reconstruction:Proto-Germanic/frijaz",
                    90,
                    92299146,
                    "==Proto-Germanic==\n{{etymon|gem-pro|id=free|:inh|ine-pro:*priHós}} & more",
                ),
                ("Reconstruction:Proto-Germanic/empty", 93, 7, ""),
            ],
        )


if __name__ == "__main__":
    unittest.main()
