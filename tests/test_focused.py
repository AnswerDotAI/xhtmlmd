"Focused dialect/math/tagfilter behavior (ported from tests/focused.rs)."
from pathlib import Path

from xhtmlmd import to_xhtml
from test_conformance import normalize_html

FIX = Path(__file__).parent / "fixtures"

def assert_html(actual, expected): assert normalize_html(actual) == normalize_html(expected)

def test_dialect_fixture():
    md = (FIX / "dialect.md").read_text()
    expected = (FIX / "dialect.xhtml").read_text()
    assert_html(to_xhtml(md), expected)

def test_flanking_treats_unicode_punctuation_as_punctuation():
    html = to_xhtml("(“***{{company_common_name}}***”)")
    assert "“<em><strong>{{company_common_name}}</strong></em>”" in html
    html = to_xhtml("“__{{x}}__”")
    assert "“<strong>{{x}}</strong>”" in html

def test_default_math_mode_is_brackets():
    html = to_xhtml("\\(y\\)\n\n\\[\nx^2\n\\]\n\n$x$")
    assert '<span class="math inline">y</span>' in html
    assert '<div class="math display">x^2</div>' in html
    assert "<p>$x$</p>" in html

def test_brackets_mode_recognizes_double_dollars():
    html = to_xhtml("$$\nx^2\n$$\n\ninline $$y$$ but not $z$")
    assert '<div class="math display">x^2</div>' in html
    assert '<span class="math display">y</span>' in html
    assert "$z$" in html

def test_single_dollar_scan_stops_at_display_dollars():
    html = to_xhtml("$x$ costs $5 and $$d$$", math="dollars")
    assert '<span class="math inline">x</span>' in html
    assert "$5 and" in html
    assert '<span class="math display">d</span>' in html

def test_brackets_double_dollar_fast_path():
    html = to_xhtml("plain words $$y$$ more")
    assert '<span class="math display">y</span>' in html

def test_math_modes_are_explicit():
    html = to_xhtml("$x$ and \\(y\\)", math="off")
    assert 'class="math' not in html
    assert "$x$ and (y)" in html
    html = to_xhtml(r"$x$ and \(y\) and \[z\]", math="on")
    assert 'class="math' not in html
    assert_html(html, "<p>$x$ and \\(y\\) and \\[z\\]</p>")
    assert_html(to_xhtml("\\[\nx^2\n\\]\n", math="on"), "<p>\\[\nx^2\n\\]</p>")
    html = to_xhtml("$x$ and \\(y\\)", math="brackets")
    assert "$x$" in html
    assert '<span class="math inline">y</span>' in html
    assert '<span class="math inline">x</span>' in to_xhtml("$x$", math="dollars")

def test_footnotes_section_has_no_hr():
    html = to_xhtml("A note[^1].\n\n[^1]: The def.\n")
    assert '<section class="footnotes" role="doc-endnotes">\n<ol>' in html
    assert "<hr" not in html

def test_tagfilter_is_opt_in():
    inp = "No <textarea>.\n\n<script>alert(1)</script>"
    default = to_xhtml(inp)
    assert "<textarea>" in default
    assert "<script>" in default
    filtered = to_xhtml(inp, tagfilter=True)
    assert "&lt;textarea>" in filtered
    assert "&lt;script>" in filtered
    assert "&lt;/script>" in filtered

def test_balance_closes_unclosed_raw_html():
    html = to_xhtml("<div>\nAfter\n", balance=True)
    assert html.count("<div>") == 1 and html.count("</div>") == 1
    assert html.rstrip().endswith("</div>")
    assert "</div>" not in to_xhtml("<div>\nAfter\n")

def test_balance_drops_stray_closes():
    html = to_xhtml("Text\n\n</div>\n", balance=True)
    assert "</div>" not in html
    assert "<p>Text</p>" in html

def test_balance_keeps_cross_block_html_pairs():
    inp = "<div>\n\n*md*\n\n</div>\n"
    assert_html(to_xhtml(inp, balance=True), to_xhtml(inp))

def test_balance_closes_mismatched_interleave():
    html = to_xhtml("<div><span>\nx\n\n", balance=True)
    assert html.rstrip().endswith("</span>\n</div>") or html.rstrip().endswith("</span></div>")

def test_balance_interleaved_close_implies_inner_close():
    html = to_xhtml("<div><span>a</div>\n", balance=True)
    assert "</span></div>" in html

def test_balance_ignores_rawtext_and_voids():
    html = to_xhtml("<script>let s = '<div>';</script>\n\n<br>\n", balance=True)
    assert "</div>" not in html
    assert "<br />" in html

def test_long_nonascii_words_near_autolink_cap_do_not_error():
    for boundary in ("(", "a: ", "x '"):
        for count in (126, 127, 128, 129, 130, 200):
            inp = boundary + "é" * count + " _x_"
            html = to_xhtml(inp)
            assert "é" * count in html
            assert "<em>x</em>" in html

def test_attr_gate_requires_marker():
    # Pandoc-style marker-first bodies attach
    assert_html(to_xhtml('# H {#h .c}\n'), '<h1 id="h" class="c">H</h1>')
    assert_html(to_xhtml('![x](/i.png){width="50%"}\n'),
                '<figure><img src="/i.png" alt="x" width="50%" /><figcaption>x</figcaption></figure>')
    # kramdown colon forms attach, including pure ALD references
    expected = '<p id="id" class="cls">Some text</p>\n'
    assert_html(to_xhtml('{:note: #id .cls}\n\nSome text\n{:note}\n'), expected)
    assert_html(to_xhtml('{:note: #id .cls}\n\nSome text\n{: note}\n'), expected)
    assert '<span id="id" class="cls">word</span>' in to_xhtml('{:note: #id .cls}\n\nA [word]{: note} here\n')
    # a colon-marked list with an unknown reference is still an attr list (consumed, ref ignored)
    assert_html(to_xhtml('Some text\n{: nope}\n'), '<p>Some text</p>')
    # bare-word bodies stay literal, even when a word matches an ALD name
    assert_html(to_xhtml('{:note: #id .cls}\n\nSome text\n{note}\n'), '<p>Some text\n{note}</p>')
    assert_html(to_xhtml('{:note: #id .cls}\n\nSome text\n{great note}\n'), '<p>Some text\n{great note}</p>')
    assert_html(to_xhtml('{:note: #id .cls}\n\nSome text\n{note .x}\n'), '<p>Some text\n{note .x}</p>')
    assert '[word]{note}' in to_xhtml('{:note: #id .cls}\n\nA [word]{note} here\n')
    # key=value only counts when the first token is a pair
    assert_html(to_xhtml('Text\n{foo k=1}\n'), '<p>Text\n{foo k=1}</p>')

def test_emphasis_strong_strike_trailing_attrs():
    assert_html(to_xhtml('a **x**{.c} b\n'), '<p>a <strong class="c">x</strong> b</p>')
    assert_html(to_xhtml('a *x*{: .c} b\n'), '<p>a <em class="c">x</em> b</p>')
    assert_html(to_xhtml('a ~~x~~{#i .c} b\n'), '<p>a <del id="i" class="c">x</del> b</p>')
    assert_html(to_xhtml('a ***x***{.c} b\n'), '<p>a <em class="c"><strong>x</strong></em> b</p>')
    assert_html(to_xhtml('{:note: .cls}\n\na **x**{: note} b\n'), '<p>a <strong class="cls">x</strong> b</p>')
    # bare words and non-adjacent braces stay literal
    assert_html(to_xhtml('a **x**{note} b\n'), '<p>a <strong>x</strong>{note} b</p>')
    assert_html(to_xhtml('a **x** {.c} b\n'), '<p>a <strong>x</strong> {.c} b</p>')

def test_table_ial_line_attaches():
    html = to_xhtml('a|b\n-|-\n1|2\n{: .c}\n')
    assert html.startswith('<table class="c">')
    assert '{: .c}' not in html

def test_raw_attribute_blocks_and_inlines():
    html = to_xhtml('```{=docx}\n<w:br w:type="page"/>\n```\n')
    assert html == '<script type="text/x-docx">&lt;w:br w:type="page"/&gt;\n</script>\n'
    html = to_xhtml('a `<w:br/>`{=docx} b')
    assert html == '<p>a <script type="text/x-docx">&lt;w:br/&gt;</script> b</p>\n'
    html = to_xhtml('```{=my-fmt_2}\nx\n```\n')  # names: alphanumeric plus - and _
    assert 'type="text/x-my-fmt_2"' in html
    # Not raw: empty name, extra info tokens, space before brace, bad name chars
    assert 'script' not in to_xhtml('```{=}\nx\n```\n')
    assert 'script' not in to_xhtml('```python {=docx}\nx\n```\n')
    assert 'script' not in to_xhtml('a `x` {=docx} b')
    assert 'script' not in to_xhtml('a `x`{=a b} c')

def test_ref_syntax():
    html = to_xhtml('see [@sec-pay] here')
    assert '<a href="#sec-pay" data-ref="data-ref"></a>' in html
    html = to_xhtml('in [-@sec-pay], the')
    assert '<a href="#sec-pay" data-ref="bare"></a>' in html
    html = to_xhtml('per [Clause @sec-pay].')
    assert '<a href="#sec-pay" data-ref="data-ref">Clause</a>' in html
    html = to_xhtml('the terms in [@sec-a; @sec-b; @sec-c] survive')
    assert ('<span class="refs"><a href="#sec-a" data-ref="data-ref"></a>'
            '<a href="#sec-b" data-ref="data-ref"></a>'
            '<a href="#sec-c" data-ref="data-ref"></a></span>') in html
    html = to_xhtml('see [-@sec-pay]{ref=page}')
    assert '<a href="#sec-pay" data-ref="bare" ref="page"></a>' in html

def test_ref_non_matches():
    assert 'data-ref' not in to_xhtml('mail [user@host] today')     # no space before @
    assert 'data-ref' not in to_xhtml('odd [@sec x] here')          # id stops at space
    assert 'data-ref' not in to_xhtml('a [@sec-a; Two @sec-b] b')   # prefix only allowed solo
    assert '<a href="/u">@sec-x</a>' in to_xhtml('[@sec-x](/u)')    # links win
    assert '[@]' in to_xhtml('empty [@] ref')

def test_para_attrs_ial_only():
    assert to_xhtml('text {.x}') == '<p>text {.x}</p>\n'            # same-line para attrs are gone
    assert to_xhtml('text\n{: .x}') == '<p class="x">text</p>\n'    # glued IAL below binds
    assert to_xhtml('{: .x}\ntext') == '<p class="x">text</p>\n'    # glued IAL above binds
    html = to_xhtml('text\n\n{: .x}')                               # isolated IAL is literal
    assert '<p>{: .x}</p>' in html and 'class' not in html
    html = to_xhtml('one\n\n{: .x}\n\ntwo')
    assert '<p>{: .x}</p>' in html and 'class' not in html
    assert to_xhtml('# H {.x}') == '<h1 id=\"h\" class=\"x\">H</h1>\n'         # headings keep same-line attrs

def test_table_captions():
    html = to_xhtml('| a |\n|---|\n| 1 |\n: My caption {#tbl-x}')
    assert '<table id="tbl-x">' in html and '<caption>My caption</caption>' in html
    html = to_xhtml('+---+\n| a |\n+---+\n: Grid cap *em* {.wide}')
    assert 'class="wide"' in html and '<caption>Grid cap <em>em</em></caption>' in html
    assert '<caption>' not in to_xhtml('| a |\n|---|\n\n: Not a caption')   # blank line: no attach
    assert '<caption>' not in to_xhtml('| a |\n|---|\n::: x\n:::')          # fenced div, not caption

def test_inline_notes():
    html = to_xhtml('Fact.^[With a *note*.]')
    assert 'class="footnote-ref"' in html and '<em>note</em>' in html and 'class="footnotes"' in html
    html = to_xhtml('A.^[one] B.[^x]\n\n[^x]: two')
    assert html.count('<li id=') == 2

def test_auto_ids():
    html = to_xhtml('# Hello World\n\n## Hello World\n\n### Fancy: Stuff! {#kept}')
    assert '<h1 id="hello-world">' in html and '<h2 id="hello-world-1">' in html and '<h3 id="kept">' in html
    assert 'id=' not in to_xhtml('# Hello', auto_ids=False)

def test_smart_punctuation():
    html = to_xhtml('"Quotes" --- em -- en ... done. Don\'t touch `--code--`.', smart=True)
    assert '“Quotes” — em – en … done' in html
    assert 'Don’t' in html and '--code--' in html
    assert '--' in to_xhtml('a -- b')   # off by default

def test_implicit_figures():
    html = to_xhtml('![A cap](i.png){#fig-x .wide}')
    assert '<figure id="fig-x" class="wide">' in html and '<figcaption>A cap</figcaption>' in html
    assert '<p>text <img' in to_xhtml('text ![A cap](i.png)')   # not alone: no figure
