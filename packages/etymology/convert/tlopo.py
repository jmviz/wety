"""TLOPO's CLDF release to assertions (Ross, Pawley and Osmond, The Lexicon of Proto Oceanic).

A pyetymdict source (`etymdict`): a cognate set reconstructs at its Level and lists
reconstructions at intermediate levels among its members, each followed by its reflexes
(PAn *Rumaq, POc *Rumaq, POc *qa-Rumaq). Its cf groups carry no category; they are
endorsed at "possible", with their Name kept as the source's words where it is prose.
Links follow the nested rule on TLOPO's tree of Oceanic, extended above it and to the
non-Oceanic languages in `etc/tree.json`.
"""

from etymology.convert import SOURCES
from etymology.convert.etymdict import Converter, Release, main_for

NAME = "tlopo"
CLDF = SOURCES / NAME / "raw/tlopo-0.1.1/cldf"


class TlopoRelease(Release):
    def __init__(self):
        super().__init__(NAME, CLDF)


class TlopoConverter(Converter):
    NAME = NAME
    LABEL = "TLOPO"
    EDITORS = "agent:tlopo-editors"
    EDITORS_NAME = "Malcolm Ross, Andrew Pawley and Meredith Osmond (TLOPO's editors)"
    IMPLIED = "implied by TLOPO's tree"
    YEAR = 2016


def main(argv=None):
    main_for(TlopoConverter, TlopoRelease(), argv)


if __name__ == "__main__":
    main()
