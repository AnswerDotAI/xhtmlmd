"Focused dialect/math/tagfilter behavior (ported from tests/focused.rs)."
from pathlib import Path

from xhtmlmd import to_xhtml

FIX = Path(__file__).parent / "fixtures"

def _norm(s): return "\n".join(l.rstrip() for l in s.splitlines()).strip()

def test_dialect_fixture():
    md = (FIX / "dialect.md").read_text()
    expected = (FIX / "dialect.xhtml").read_text()
    assert _norm(to_xhtml(md)) == _norm(expected)

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
    assert html == "<p>$x$ and \\(y\\) and \\[z\\]</p>\n"
    assert to_xhtml("\\[\nx^2\n\\]\n", math="on") == "<p>\\[\nx^2\n\\]</p>\n"
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
    assert to_xhtml(inp, balance=True) == to_xhtml(inp)

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
    assert to_xhtml('# H {#h .c}\n') == '<h1 id="h" class="c">H</h1>\n'
    assert to_xhtml('![x](/i.png){width="50%"}\n') == '<p><img src="/i.png" alt="x" width="50%" /></p>\n'
    # kramdown colon forms attach, including pure ALD references
    expected = '<p id="id" class="cls">Some text</p>\n'
    assert to_xhtml('{:note: #id .cls}\n\nSome text\n{:note}\n') == expected
    assert to_xhtml('{:note: #id .cls}\n\nSome text\n{: note}\n') == expected
    assert '<span id="id" class="cls">word</span>' in to_xhtml('{:note: #id .cls}\n\nA [word]{: note} here\n')
    # a colon-marked list with an unknown reference is still an attr list (consumed, ref ignored)
    assert to_xhtml('Some text\n{: nope}\n') == '<p>Some text</p>\n'
    # bare-word bodies stay literal, even when a word matches an ALD name
    assert to_xhtml('{:note: #id .cls}\n\nSome text\n{note}\n') == '<p>Some text\n{note}</p>\n'
    assert to_xhtml('{:note: #id .cls}\n\nSome text\n{great note}\n') == '<p>Some text\n{great note}</p>\n'
    assert to_xhtml('{:note: #id .cls}\n\nSome text\n{note .x}\n') == '<p>Some text\n{note .x}</p>\n'
    assert '[word]{note}' in to_xhtml('{:note: #id .cls}\n\nA [word]{note} here\n')
    # key=value only counts when the first token is a pair
    assert to_xhtml('Text\n{foo k=1}\n') == '<p>Text\n{foo k=1}</p>\n'

def test_emphasis_strong_strike_trailing_attrs():
    assert to_xhtml('a **x**{.c} b\n') == '<p>a <strong class="c">x</strong> b</p>\n'
    assert to_xhtml('a *x*{: .c} b\n') == '<p>a <em class="c">x</em> b</p>\n'
    assert to_xhtml('a ~~x~~{#i .c} b\n') == '<p>a <del id="i" class="c">x</del> b</p>\n'
    assert to_xhtml('a ***x***{.c} b\n') == '<p>a <em class="c"><strong>x</strong></em> b</p>\n'
    assert to_xhtml('{:note: .cls}\n\na **x**{: note} b\n') == '<p>a <strong class="cls">x</strong> b</p>\n'
    # bare words and non-adjacent braces stay literal
    assert to_xhtml('a **x**{note} b\n') == '<p>a <strong>x</strong>{note} b</p>\n'
    assert to_xhtml('a **x** {.c} b\n') == '<p>a <strong>x</strong> {.c} b</p>\n'

def test_table_ial_line_attaches():
    html = to_xhtml('a|b\n-|-\n1|2\n{: .c}\n')
    assert html.startswith('<table class="c">')
    assert '{: .c}' not in html

