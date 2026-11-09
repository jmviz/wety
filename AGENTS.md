# Agent instructions

These rules apply to every agent working on wety, in any harness. The maintainer
supervises many sessions with limited time: work must be easy to follow, and its state
must live where every session and the maintainer can see it.

## Where things are

[README.md](README.md) explains what wety is and how the repository is organised. The
dating contract is [packages/dating/README.md](packages/dating/README.md); work inside
`packages/dating/` also follows [packages/dating/AGENTS.md](packages/dating/AGENTS.md).

## The work tracker

Work in progress, questions for the maintainer and ideas not yet started live in a
private tracker repository (`jmviz/wety-work`), cloned at `./.work` in every checkout and
ignored by wety. Its `README.md` is the board; its `FORMAT.md` has the formats in full.

At the start of a session:

1. If `.work/` is missing, clone it:

   ```sh
   main=$(git worktree list --porcelain | sed -n '1s/^worktree //p')
   git clone "$main/.work" .work 2>/dev/null || git clone git@github.com:jmviz/wety-work.git .work
   git -C .work remote set-url origin git@github.com:jmviz/wety-work.git
   git -C .work config core.hooksPath hooks
   ```

   Without access to the private repository (some cloud sessions), skip the tracker;
   your branch still shows on the board.
2. `git -C .work pull -q --rebase`.
3. If a stream names your branch (`grep -l "^branch: $(git branch --show-current)$"
   .work/streams/*.md`), read it and the inbox items addressed to it
   (`grep -l "^for: <slug>$" .work/inbox/*.md`; act on them, then delete them), and
   raise its *Needs you* questions when the maintainer is present.

While working:

- Work that will take more than one sitting gets a stream file
  (`.work/streams/<slug>.md`, from `.work/templates/stream.md`). A one-sitting change
  needs none: its commits are its record.
- Keep your stream's *Now* current and tick its steps at each checkpoint. Only the
  session working on a stream edits its file, except for edits the maintainer asks for.
- Anything noticed but not done now goes into a new inbox file
  (`.work/inbox/<yyyy-mm-dd>-<slug>-<3 random chars>.md`), not into your stream.
- When the maintainer says "note …" or reports a bug or idea in passing, write it as an
  inbox file. When they ask for the board, run `python3 .work/bin/board` and show it.
  When they answer a question, record the answer in that stream's file.
- After each tracker change: commit only the files you changed, then
  `git -C .work pull -q --rebase && git -C .work push -q`.

Questions for the maintainer:

- A question at the end of a turn stays in the conversation.
- A question that blocks a stream beyond this session goes under its *Needs you*,
  dated, with the default you will follow if it is not answered. Decide what you can
  decide yourself; ask only what you cannot.
- Settled questions are removed; anything lasting goes into the stream's text, or into
  wety's docs or these instructions if it is a project rule.

At the end of a session, commit and push both wety and the tracker; leave nothing
uncommitted. A stream ready for the maintainer to land is `state: review`.

## Scope and approval

You may proceed without asking on work the maintainer asked for in the session, bug
fixes, tests, documentation, and research within an approved stream's scope.

Write a `proposed` stream and ask before starting: a new research direction or family,
any model, backend or harness comparison, changes to repository layout, provenance
machinery or workflow infrastructure, and deleting anything under `data/`.

Every stream states its budget and when to stop. If the budget runs out or the work
stalls, stop and record the precise obstacle in *Now*.

## Git

- Work on a branch, never directly on `main`. Never merge into or push `main` or
  `production`: pushing `main` deploys the API server and then the website
  ([deploy/README.md](deploy/README.md)); the maintainer does that. A pre-push hook
  refuses such pushes without `WETY_DEPLOY=1`; never set it.
- Do not push to `origin`. All local branches are backed up to a private mirror
  automatically.
- Commit at coherent checkpoints and always before ending a session.
- Stage specific paths and inspect what is staged. Never commit source PDFs or other
  publisher content, logs, model transcripts, caches or generation bundles. A pre-push
  guard refuses some of these; do not rely on it.
- Research outputs (evidence, ledger, runs, packets, receipts, retained texts) never
  enter git: they live in the research store. Git gets code, docs, the files the app is
  built from, run READMEs and briefs (the exact list is `.gitignore`). Changing that
  list is a maintainer decision.

## The research store and worktrees

Most of `data/` is ignored: the research store (artifacts, caches, evidence, runs) lives
in the main checkout, the first entry of `git worktree list`
([provenance](docs/architecture/provenance.md)).

- Reading only: use the main checkout's files by absolute path.
- Running a workflow from a worktree: the runner rejects inputs outside the repository,
  symlinks included, so hard-link each file named in the workflow's `inputs` (`ln`, same
  volume). Paths passed only as command arguments may be symlinks, e.g.
  `data/cache/embeddings/current` (one graph build at a time may use it). Commit first:
  the receipt records the revision and whether the tree was dirty. Afterwards copy the
  run directory and artifact into the main checkout's `data/` (`cp -Rc`). The runner
  also rewrites the tracked `data/products/*/CURRENT.json`; commit that only to adopt
  the output, else restore it.
- At each checkpoint, bring a worktree's new data into the main checkout
  (`uv run --frozen wety-data store sync data <main>/data --apply`; it never
  overwrites; report its conflicts), then snapshot from the main checkout
  (`wety-data store snapshot`) and commit `data/SNAPSHOT.json`. If
  `wety-data store status` shows no `settings`, the store is not configured here: say so.
- Without the main checkout, restore `data/` from the store (`wety-data store restore`,
  then `sync`); if the store is not configured either, report it rather than
  re-acquiring sources. Do not pipe restic into `head` (it can leave a stale lock).
- Serving another graph: set `WETY_GRAPH_PATH`; one graph needs about 5 GB of memory.

## The local site

The maintainer checks work in a local API server (`target/release/server`, port 3000,
about five minutes to load a graph) and the Vite dev client (`client/`, port 8000 or the
next free one, against the local API through `.env.development`). Leave them running
between turns: start them detached (`nohup … &`), restart the server after Rust
changes, and do not stop them at the end of a task.

## Documents

- When you learn something the next person needs, write it where they will meet it:
  the code's doc comment, the tool's output or error message, the prompt the model
  reads, the component's README or docs. AGENTS.md files hold only how agents work. Do
  not use a harness's own memory store for wety: other harnesses never see it. A note
  another live branch needs before yours lands goes into an inbox item addressed to its
  stream (`for:`).
- Work state lives in the tracker, history in git. Do not write dated reports, session
  logs or next-steps documents in the repository.
- A new document replaces or merges an existing one.
- Write for a human reader. A README says what something is and how to use it, in plain
  language; internal rules and agent conventions belong in AGENTS.md files.
- Run artifacts belong in the run's directory under `data/`, not in docs.
- Link only to files git tracks; name a research-store path as code text
  (`data/workspaces/dating/_research/packets/...`).

## Code

- Match the surrounding style.
- Run the relevant tests before committing (commands in [README.md](README.md)).
- When a change replaces behaviour, remove the old behaviour. No interface, format or
  behaviour here is guaranteed unless a document says so, so fallbacks for old shapes,
  legacy aliases and dual code paths need a stated guarantee or a compelling reason,
  named in the commit. Change both sides of an interface in one commit and say what
  must be restarted or rebuilt.

## Reporting

Report results in wety terms: what a user of wety can now see or do (for example, "N
language codes gained a displayable, cited period"). Separate what is established from
what is proposed. State failures and skipped steps plainly.
