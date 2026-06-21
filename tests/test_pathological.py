"Bounded-time behavior on pathological inputs (ported from tests/pathological.rs)."
import time

from xhtmlmd import to_xhtml

BOUND = 10.0  # generous vs the Rust 5s bound; still catches super-linear blowup

def _timed(inp):
    t = time.time()
    html = to_xhtml(inp)
    return html, time.time() - t

def test_nested_brackets_do_not_explode():
    n = 50_000
    html, el = _timed("[" * n + "a" + "]" * n)
    assert "a" in html
    assert el < BOUND

def test_deep_blockquotes_are_bounded():
    n = 50_000
    html, el = _timed("> " * n + "a")
    assert "a" in html
    assert el < BOUND

def test_repeated_image_openers_are_linear_smoke():
    _, el = _timed("![[]()" * 20_000)
    assert el < BOUND

def test_raw_html_balancing_is_linear_smoke():
    n = 20_000
    html, el = _timed("<div>\n" * n + "</div>\n" * n)
    assert html.startswith("<div>")
    assert el < BOUND

def test_many_abbreviations_are_bounded():
    n = 2_000
    inp = "".join(f"*[ABBR{i}]: title {i}\n" for i in range(n)) + "\n" + "".join(f"ABBR{i} " for i in range(n))
    html, el = _timed(inp)
    assert '<abbr title="title 1999">ABBR1999</abbr>' in html
    assert el < BOUND

def test_nested_footnote_references_are_bounded():
    n = 1_000
    inp = "Start[^0]\n\n"
    for i in range(n):
        inp += f"[^{i}]: note"
        if i + 1 < n: inp += f"[^{i+1}]"
        inp += "\n\n"
    html, el = _timed(inp)
    assert "fn-999" in html
    assert el < BOUND

def test_markdown_html_close_scanning_skips_code_spans():
    n = 10_000
    inp = '<div markdown="1">\n' + "code `</div>` " * n + "\n\nstill here\n</div>"
    html, el = _timed(inp)
    assert "<p>still here</p>" in html
    assert el < BOUND
