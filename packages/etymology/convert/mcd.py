"""The MCD's CLDF release to assertions (Bender et al. 2003, Proto-Micronesian Reconstructions).

A pyetymdict source (`etymdict`): its cognate sets reconstruct Micronesian levels, with
reconstructions at lower levels and comparanda outside Micronesian among their members;
its "Cf." groups are forms listed beside a set, endorsed at "possible"; its loan groups
take their members from `borrowings.csv` and keep their description as the source's
words. Links follow the nested rule on the MCD's tree, extended above Proto-Micronesian
in `etc/tree.json`; a subgroup's reconstruction comes after its reflexes in a set.
"""

from etymology.convert import SOURCES
from etymology.convert.etymdict import Converter, Release, main_for

NAME = "mcd"
CLDF = SOURCES / NAME / "raw/lexibank-mcd-1cabc58/cldf"


class McdRelease(Release):
    def __init__(self):
        super().__init__(NAME, CLDF)


class McdConverter(Converter):
    NAME = NAME
    LABEL = "MCD"
    EDITORS = "agent:mcd-authors"
    EDITORS_NAME = (
        "Bender, Goodenough, Jackson, Marck, Rehg, Sohn, Trussel and Wang (the MCD's authors)"
    )
    IMPLIED = "implied by the MCD's tree"
    YEAR = 2003
    RECONSTRUCTION_FIRST = False  # a subgroup's reconstruction follows its reflexes


def main(argv=None):
    main_for(McdConverter, McdRelease(), argv)


if __name__ == "__main__":
    main()
