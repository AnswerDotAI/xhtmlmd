# The Most Comprehensive and Adversarial Markdown Test Suites: A Parser Developer's Field Guide

## TL;DR
- The single most valuable, machine-readable conformance corpus is **CommonMark's `spec.txt`/`spec.json`** (over 500 embedded examples; 0.31.2 is the current release), extended by **GitHub's `cmark-gfm` repo**, which adds GFM extension tests in `test/extensions.txt` and ships the genuinely adversarial **`test/pathological_tests.py`** (50,000-deep nested brackets/blockquotes, etc.) — start here.
- For **extensions** (footnotes, definition lists, attribute lists/IAL, abbreviations, tables, math) and especially the **`<div markdown="1">` markdown-in-HTML pattern**, the best-organized fixtures are **kramdown's `test/testcases/` tree**, **Python-Markdown's `tests/`** (md_in_html), and **Michel Fortin's MDTest** (`PHP Markdown Extra.mdtest/`).
- The best cross-implementation comparison tool is **Babelmark 3** (20+ parsers side-by-side); the classic monolithic "torture" documents (Gruber's original MarkdownTest, mxstbr/markdown-test-file) are useful smoke tests but are largely superseded by the structured per-feature suites above.

## Key Findings

1. **There is no single "standard" Markdown test suite.** The original community attempt (karlcow/markdown-testsuite) and Gruber/Fortin's MDTest are now declared obsolete. The CommonMark/Standard Markdown launch materials state it plainly: *"There's no standard test suite for Markdown; the unofficial MDTest is the closest thing we have. The only way to resolve Markdown ambiguities and inconsistencies is Babelmark, which compares the output of 20+ implementations of Markdown against each other to see if a consensus emerges."* The de facto standards today are CommonMark's embedded spec tests plus each parser's own fixtures.

2. **Conformance suites are spec-embedded.** CommonMark and GFM store input/output pairs *inside* the spec file itself (`spec.txt`) in a fenced shorthand, extracted to JSON for machine consumption. This is the dominant pattern, and many parsers (markdown-it, marked) vendor the CommonMark `spec.txt` directly.

3. **The hardest, most adversarial material is cmark/cmark-gfm's pathological tests** — explicitly designed to cause super-linear (quadratic/exponential) blowup and crash naive parsers. Several entries correspond to real CVEs (see Details).

4. **For the `<div markdown="1">` behavior the user called out**, three canonical sources exist: PHP Markdown Extra (the origin of the `markdown="1"` attribute), Python-Markdown's `md_in_html` extension, and kramdown's `parse_block_html` option. Each has dedicated tests, and their semantics differ.

5. **Extension coverage is richest in kramdown and Python-Markdown/PyMdown Extensions**, which test footnotes, definition lists, IAL/ALD attribute lists, abbreviations, math, and tables as discrete fixture groups.

## Details

### 1. Machine-readable conformance suites (input/expected-output pairs)

**CommonMark — the canonical baseline.**
- Repo: `github.com/commonmark/commonmark-spec`. The source of truth is **`spec.txt`**, a valid CommonMark file with examples in a fenced shorthand (` ```````example ` … `.` … ` ``````` `). Per its README, the spec *"contains over 500 embedded examples which serve as conformance tests."* The current released version is **0.31.2 (2024-01-28)** (the 0.29/0.30 generation has 649 numbered examples).
- Machine-readable form: run `python3 test/spec_tests.py --dump-tests` to emit JSON, or fetch the published `spec.json` (e.g. `spec.commonmark.org/0.30/spec.json`). Each test object is `{"markdown":..., "html":..., "section":..., "example":...}`.
- Convenience packages: `wooorm/commonmark.json` and the npm `commonmark-spec` / `commonmark.json` packages expose the tests as plain JS objects.
- Difficulty: comprehensive on *core* Markdown edge cases (emphasis precedence, list interruption, tab handling, link/entity parsing) but contains **no extensions**. This is your correctness floor, not your stress test.

**GitHub Flavored Markdown (GFM) — CommonMark + 5 extensions.**
- Spec website: `github.github.com/gfm/` (version 0.29-gfm, 2019). The implementation and tests live in **`github.com/github/cmark-gfm`**, a fork of `commonmark/cmark`.
- Main conformance file: **`test/spec.txt`** — the full GFM spec (CommonMark base plus extension sections). It is a strict superset of CommonMark.
- Extension tests: GFM extension conformance is **not** split into per-extension files; there is a single **`test/extensions.txt`** (referenced as `EXTENSIONS_SPEC=test/extensions.txt` in the Makefile), plus **`test/extensions-full-info-string.txt`**. Individual examples select which extension to enable. The five extensions are **table, strikethrough, autolink, tagfilter, tasklist** (GFM spec sections 4.10, 6.5, 6.9, 6.11, 5.3 respectively).
- Harness: `test/spec_tests.py`, `test/cmark.py`, `test/normalize.py`.
- Note: GitHub does not publish the GFM spec fork as a standalone repo — `cmark-gfm` is where `spec.txt` and the tests actually live.

**The pathological / "torture by performance" tests — the hardest adversarial material.**
- File: **`github.com/github/cmark-gfm/blob/master/test/pathological_tests.py`** (also in upstream `commonmark/cmark` and `apple/swift-cmark`). Each entry is an (input, expected-output-regex) pair run in a subprocess with a 5-second timeout; the parser is invoked with `extensions="table autolink"`.
- Notable generated inputs include: **"nested brackets"** = `("[" * 50000) + "a" + ("]" * 50000)`; **"nested block quotes"** = `("> " * 50000) + "a"`; **"deeply nested lists"** (1000 levels); `"nested strong emph"` = `("*a **a " * 65000)…`; `"many emph closers/openers"` (65000×); `"pattern [ (]( repeated"` (80000×); `"pattern ![[]() repeated"` (160000×); `"unclosed <!--"` (300000×); `"many image openers"` (100000×); plus hash-collision reference labels.
- These exploit known blowups. cmark's README (verbatim): *"The library has been extensively fuzz-tested using american fuzzy lop. The test suite includes pathological cases that bring many other Markdown parsers to a crawl (for example, thousands-deep nested bracketed text or block quotes)."*
- Concrete historical bugs the suite guards against:
  - **Issue #214** — unclosed inline links `print("[a](b" * 50000)` *"takes inadequately long time … (On my Windows machine it takes 13.076 seconds.)"*; fixed by *"limiting the parenthesis nesting level to 32."*
  - **Issue #255** — deeply nested lists *"exhibits heavily non-linear time (likely quadratic)"* with a default generator of `N = 1000`; the reporter notes *"(Interestingly and to my surprise, nested blockquotes are fine.)"*
  - **CVE-2023-22486** — *"Fix quadratic complexity bug with repeated `![[]()`."*
  - **CVE-2023-22484** — *"Fix quadratic parsing issue with repeated `<!--`."*
- **This is the single best resource for stress-testing a new parser's algorithmic robustness.** The benchmark input `bench/full-sample.md` is also noted to make pandoc and Markdown.pl "grind to a halt."

**markdown-it (JS) — vendored CommonMark + own extension fixtures.**
- Repo: `github.com/markdown-it/markdown-it`, tests under **`test/fixtures/`**. Key files: `test/fixtures/commonmark/spec.txt` (vendored CommonMark), `test/fixtures/markdown-it/strikethrough.txt`, `.../tables.txt`, and others.
- Fixture format is the commonmark-spec shorthand, parsed by the separate **`markdown-it/markdown-it-testgen`** package (also forked as `GerHobbelt/markdown-it-testgen`), which supports YAML front-matter metadata per fixture file and `.`-separated source/result blocks. A clean, reusable harness for testing plugins.

**marked (JS).**
- Repo: `github.com/markedjs/marked`. Tests live under `test/specs/`, split into `commonmark/` and `gfm/` folders (the project imports the CommonMark `spec.json` and the GFM spec). A separate **`markedjs/testutils`** package exposes the spec runner (`Tests`/`Spec` interfaces, `shouldFail` flags, completion-percentage tables) so extensions can run the full marked spec suite.

**Python-Markdown.**
- Repo: `github.com/Python-Markdown/markdown`, tests under **`tests/`**. Uses two harness styles documented at `python-markdown.github.io/test_tools/`: `markdown.test_tools.TestCase` (inline source/expected) and `LegacyTestCase` (paired `.txt`/`.html` files in a directory, one unit test per pair, optional HTML normalization via PyTidyLib). Per-file keyword args (extensions, output format) are configured via `Kwargs`.

**PyMdown Extensions (facelessuser/pymdown-extensions).**
- Tests under **`tests/extensions/`**: each is a Markdown text file converted with extensions/options defined in `tests/extensions/tests.yml`, compared against stored HTML. Covers a large bundle of advanced extensions (superfences, arithmatex math, etc.).

**kramdown (Ruby) — the richest extension fixture tree.**
- Repo: `github.com/gettalong/kramdown`. Tests under **`test/testcases/`**, driven by `test/test_files.rb`. Each case is a **`.text` input + `.html` expected-output pair** sharing a base name, optionally with a sibling **`.options`** file (YAML conversion options) and version/round-trip variants.
- `test/testcases/block/` subfolders include: `03_paragraph`, `04_header`, `05_blockquote`, `06_codeblock` (with a `rouge/` subdir), `08_list`, `09_html` (with `html_to_native/`), `10_ald` (Attribute List Definitions), `11_ial` (block Inline Attribute Lists), `12_extension`, `13_definition_list`, `14_table`, `15_math`, `16_toc`.
- `test/testcases/span/` subfolders include: `01_link`, `02_emphasis`, `03_codespan`, `05_html`, `ial` (span IAL), `math`, `abbreviations`, `text_substitutions`, `extension`.
- kramdown is also reusable cross-implementation: mirrored in `plexus/kramberry` (`resources/kramdown_test_suite/`) and `digitalmoksha/motion-kramdown`.

**Pandoc (Haskell).**
- Repo: `github.com/jgm/pandoc`. Test suite code in **`test/test-pandoc.hs`**; reader/writer data files under `test/` (modify `test/Tests/Old.hs` to register new file-based tests). Run with `cabal test --test-options='-p markdown'`. Many of pandoc's markdown tests are "adapted from John Gruber's markdown test suite." Pandoc also defines `markdown_phpextra` and `markdown_mmd` reader variants worth testing against.

**MultiMarkdown.**
- Repo: `github.com/fletcher/MMD-Test-Suite`. Structure: `Tests/` = the original Gruber suite (run with `multimarkdown -c` compatibility mode); `MultiMarkdownTests/` = XHTML and LaTeX output tests; `BeamerTests/` and `MemoirTests/` = output-mode-specific. Driven by a modified `MarkdownTest.pl` with a `--Flags` extension to pass output-format flags.

**Maruku (Ruby, obsolete).**
- Repo: `github.com/bhollis/maruku`. Tests in **`spec/block_docs`** (run via `bundle exec rake`); superseded by kramdown but still maintained for bug reports.

### 2. PHP Markdown Extra and the `<div markdown="1">` markdown-in-HTML pattern

This is the feature the user explicitly flagged, and it has three canonical homes — whose behaviors differ:

**PHP Markdown Extra (origin of the `markdown="1"` attribute).**
- Repo: `github.com/michelf/php-markdown`. Tests were historically in the separate **MDTest** repo (`github.com/michelf/mdtest`) but are now bundled in the source tree under `test/resources/php-markdown-extra.mdtest/`.
- MDTest structure: tests are grouped into `.mdtest` folders — `Markdown.mdtest/` (Gruber's original Markdown.pl tests), `PHP Markdown.mdtest/`, and **`PHP Markdown Extra.mdtest/`**. Each test is a `.text` input + `.xhtml` expected-output pair. The Extra folder's files directly cover the requested extensions: `Abbr`, `Definition Lists`, `Emphasis`, `Footnotes`, `Headers with attributes`, **`Inline HTML with Markdown content`** (the `markdown="1"` test), `Link & Image Attributes`, `Tables`, and `Backtick`/`Tilde Fenced Code Blocks`.
- Run via `./mdtest.php -f \\Michelf\\Markdown::defaultTransform`. The "Inline HTML with Markdown content" case is the canonical `markdown="1"` fixture.

**Python-Markdown `md_in_html` extension.**
- Docs: `python-markdown.github.io/extensions/md_in_html/`; source at `markdown/extensions/md_in_html.py`. Explicitly *"Based on the implementation in PHP Markdown Extra."* The `markdown` attribute accepts `"1"`, `"block"`, or `"span"`. Tests live in Python-Markdown's `tests/` tree (the md_in_html fixtures).
- Behavioral edge cases worth testing (from the docs): unclosed `<p>` tags get normalized/closed; nested elements with `markdown` attributes; tags always ignored regardless of attribute (`canvas, math, option, pre, script, style, textarea`); default block tags (`article, aside, blockquote, div, details, section, table`, etc.).

**kramdown `parse_block_html` + `markdown` attribute.**
- Tested under **`test/testcases/block/09_html/`**, specifically the **`parse_block_html.text` / `parse_block_html.html`** pair, plus the `html_to_native/` subdir. kramdown's global `parse_block_html` option (default `false`) enables markdown parsing inside block HTML; per-tag control via the `markdown="1"/"0"/"block"/"span"` attribute.

### 3. Extension-specific coverage map

| Feature | Best canonical fixtures |
|---|---|
| **GFM tables** | cmark-gfm `test/spec.txt`/`extensions.txt`; markdown-it `test/fixtures/markdown-it/tables.txt` |
| **PHP Extra / MMD / kramdown tables** | php-markdown `PHP Markdown Extra.mdtest/Tables.text`; MMD-Test-Suite; kramdown `block/14_table/` |
| **Raw HTML + `markdown="1"`** | php-markdown "Inline HTML with Markdown content"; Python-Markdown md_in_html tests; kramdown `block/09_html/parse_block_html` |
| **Footnotes** | php-markdown `Footnotes`; kramdown (footnote cases); Python-Markdown footnotes tests |
| **Definition lists** | php-markdown `Definition Lists`; kramdown `block/13_definition_list/` |
| **Attribute lists / IAL / ALD** | kramdown `block/10_ald`, `block/11_ial`, `span/ial`; php-markdown `Headers with attributes`, `Link & Image Attributes` |
| **Abbreviations** | php-markdown `Abbr`; kramdown `span/abbreviations/` |
| **Task lists / strikethrough / autolinks** | cmark-gfm extensions; markdown-it `strikethrough.txt` |
| **Math / LaTeX** | kramdown `block/15_math`, `span/math`; PyMdown arithmatex tests |
| **Fenced code + info strings** | cmark-gfm `extensions-full-info-string.txt`; php-markdown Backtick/Tilde Fenced Code Blocks |
| **Front matter / TOC** | kramdown `block/16_toc`; marked/markdown-it front-matter plugins |

### 4. Cross-implementation comparison tools

**Babelmark 3** (`babelmark.github.io`) — the current best tool. Runs a fragment through 20+ Markdown implementations across many languages and shows outputs side-by-side. Architecture: `babelmark/babelmark.github.io` (SCSS front-end), `babelmark/babelmark-registry` (registry of implementation endpoints), `babelmark/babelmark-proxy` (C# proxy that dispatches/aggregates). commonmark.org: *"The best current way to resolve Markdown ambiguities and inconsistencies is Babelmark 3, which compares the output of 20+ implementations of Markdown against each other to see if a consensus emerges."* (Jeff Atwood famously counted *"fifteen different rendered outputs from 22 different Markdown parsers."*) A classic divergence game: the smallest strings that split parsers, e.g. `*a**b*`, `_a_b`, `# cup`.

**Babelmark 2** (`johnmacfarlane.net/babelmark2/`) — the older predecessor, still referenced.

**karlcow/markdown-testsuite** — an archived community attempt at a language-agnostic suite with a Vagrant-provisioned harness running many engines (blackfriday, gfm, hoedown, kramdown, lunamark, markdown.pl, markdown2, marked). Now closed in favor of CommonMark, but its results tables remain instructive (e.g. lunamark failing ~51% of cases, kramdown ~22%, marked failing a long table-related run).

**elucent/mdtest** — a *different* tool (not Fortin's): a Markdown-based unit-test harness that executes embedded code blocks; useful for literate testing, not parser conformance. Do not confuse it with `michelf/mdtest`.

### 5. Monolithic "torture" / smoke-test documents

These are single sprawling documents (not input/output pairs), good for visual regression and quick smoke tests:
- **mxstbr/markdown-test-file** (`TEST.md`) — based on Gruber's Markdown syntax page with HTML stripped; the most-cited "does my renderer basically work" file.
- **Gruber's original MarkdownTest** — the 2004 ancestor of all the above, embedded in MMD-Test-Suite `Tests/` and MDTest `Markdown.mdtest/`.
- **Various "full markdown example" gists** (allysonsilva, extratone) covering headings, emphasis, tables, strikethrough, task lists in one file.
- **Zettelkasten-Method/10000-markdown-files** — 10,000 files for stress-testing note-taking tools (volume, not edge-case depth).

## Recommendations

**Stage 1 — Establish a correctness floor (do this first).**
1. Vendor **CommonMark `spec.json`** (extract via `spec_tests.py --dump-tests` or fetch from `spec.commonmark.org/0.30/spec.json`) and wire it into CI as a full pass/fail matrix. This is non-negotiable for any CommonMark-aspiring parser.
2. Add **cmark-gfm `test/spec.txt` + `test/extensions.txt`** if you target GFM. Reuse `markdown-it-testgen` (JS) or `spec_tests.py` (Python) as a ready-made runner so you don't write your own fixture parser.

**Stage 2 — Harden against adversarial input.**
3. Run **`cmark-gfm/test/pathological_tests.py`** against your parser with the same 5-second timeout. If any case exceeds linear-ish time (watch the nested-list and nested-bracket cases, and the `![[]()` / `<!--` repetition cases tied to CVE-2023-22486 and CVE-2023-22484), you have a quadratic bug. Benchmark: the deeply-nested-lists case at the default N=1000 should finish well under a second; doubling N must not blow up super-linearly.
4. Fuzz with american fuzzy lop or libFuzzer (cmark ships a libFuzzer target) — the pathological suite catches *known* bugs; fuzzing catches the unknown ones.

**Stage 3 — Extensions and the `markdown="1"` behavior.**
5. For markdown-in-HTML, test against **all three** reference behaviors, because they differ: php-markdown's "Inline HTML with Markdown content" fixture, Python-Markdown's `md_in_html` `tests/`, and kramdown's `block/09_html/parse_block_html`. Decide which semantics you're matching and pin it explicitly.
6. Pull per-feature fixtures from **kramdown `test/testcases/`** (best-organized for footnotes, IAL/ALD, definition lists, math, tables) and **php-markdown `PHP Markdown Extra.mdtest/`** (footnotes, abbr, attributes).

**Stage 4 — Cross-validation and visual regression.**
7. Use **Babelmark 3** interactively whenever you hit an ambiguous construct — compare your output against the 20+ implementations to see if a consensus exists before deciding behavior.
8. Add **mxstbr/markdown-test-file** as a one-shot visual smoke test in your demo/preview pipeline.

**Benchmarks that should change your decisions:**
- If a pathological case times out → fix the algorithm before adding features (algorithmic correctness > feature breadth).
- If you diverge from Babelmark consensus on a core construct → align with CommonMark unless you have a documented reason.
- If you only need core Markdown → skip GFM/kramdown extension suites entirely; they'll generate noise.

## Caveats
- **"Obsolete" ≠ useless.** MDTest, karlcow/markdown-testsuite, and Maruku are formally deprecated, but their fixtures (especially PHP Extra's `markdown="1"` cases and Gruber's originals) remain the *canonical* reference for pre-CommonMark behavior. The CommonMark project itself calls MDTest the "closest thing we have," yet it is still the origin of much extension test material.
- **The GFM `spec.txt` example count is approximate.** The CommonMark base (0.29/0.30 generation) is 649 examples; the released 0.31.2 README says "over 500." The GFM additions bring the total to roughly 670–680, but this was not line-counted from the live file — verify by counting fences in the raw `test/spec.txt` if an exact figure matters.
- **cmark-gfm uses one `test/extensions.txt`, not per-extension files.** Per-extension behavior is selected per-example via the extensions field, so don't expect a `tables.txt`/`strikethrough.txt` split there (markdown-it *does* split them).
- **Extensions are not standardized.** `markdown="1"`, footnotes, IAL, math, and front matter all behave differently across PHP Extra, kramdown, Python-Markdown, and Pandoc. There is no cross-implementation conformance suite for extensions — you must pick a reference dialect per feature.
- **Monolithic torture documents test breadth, not correctness.** They render "successfully" even when subtly wrong because there's no expected-output to diff against. Use them for smoke tests only; use input/output pairs for real conformance.
- A handful of the lowest-numbered kramdown block folders (e.g. blank-line, EOB, horizontal-rule) follow the same numbering scheme but were not individually verified; confirm against the live `test/testcases/block/` listing.