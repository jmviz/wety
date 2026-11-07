import { JSX } from "solid-js";

export const GITHUB = "https://github.com/jmviz/wety";

const Ext = (props: { href: string; children: JSX.Element }) => (
  <a href={props.href} target="_blank" rel="noopener noreferrer">
    {props.children}
  </a>
);

// What wety is, what it draws from, and where its code lives.
export default function About() {
  return (
    <article class="about">
      <h1>About wety</h1>
      <p>
        wety, for <i>(w)iktionary (ety)mologies</i>, draws the etymologies recorded in
        Wiktionary as trees: where a word came from, what came from it, and the words that
        share its ancestors.
      </p>

      <h2>Reading it</h2>
      <ul>
        <li>
          <b>Etymology</b> charts a word's sources, oldest at the top. A dashed link is
          possible or doubtful; an underlined mode is wety's inference, not a source's
          statement.
        </li>
        <li>
          <b>Descendants</b> lists the words that came from it, as an outline.
        </li>
        <li>
          <b>Cognates</b> traces the word back as far as its sources go, then draws that
          ancestor's descendants. Any word on the trail above the tree can start it instead.
        </li>
      </ul>
      <p>
        Hover over a word, or tap it, for its senses, its line of descent and its origin:
        who states it, on which page, and the works they follow. A language's dates open
        from its name, each with its citations.
      </p>

      <h2>Sources</h2>
      <ul>
        <li>
          <Ext href="https://en.wiktionary.org/">English Wiktionary</Ext>, by its editors,
          under <Ext href="https://creativecommons.org/licenses/by-sa/4.0/">CC BY-SA 4.0</Ext>.
          Every word links to its entry.
        </li>
        <li>
          Michiel de Vaan's <i>Etymological Dictionary of Latin and the other Italic
          Languages</i> (Brill, 2008), as published in the{" "}
          <Ext href="https://lila-erc.eu/">LiLa</Ext> Knowledge Base, under{" "}
          <Ext href="https://creativecommons.org/licenses/by-nc-sa/4.0/">CC BY-NC-SA 4.0</Ext>.
        </li>
        <li>
          Robert Blust and Stephen Trussel's{" "}
          <Ext href="https://acd.clld.org/">Austronesian Comparative Dictionary</Ext>, under{" "}
          <Ext href="https://creativecommons.org/licenses/by/4.0/">CC BY 4.0</Ext>.
        </li>
      </ul>
      <p>
        Where these sources disagree about a word, its details say so, and how one reading
        was preferred. The dates of languages come from published sources, each period with
        its citations.
      </p>
      <p>
        Wiktionary is written by volunteers, and wety reads it by rule, so some links are
        wrong or missing. Corrections to an etymology belong on Wiktionary, where they will
        reach wety with its next update.
      </p>

      <h2>Code</h2>
      <p>
        wety is open source, under the MIT licence, on{" "}
        <Ext href={GITHUB}>GitHub</Ext>; problems with wety itself can be reported there.
        It is set in{" "}
        <Ext href="https://brill.com/page/resources_fontsdownloads">Brill</Ext>.
      </p>
    </article>
  );
}
