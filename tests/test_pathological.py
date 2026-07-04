"Bounded-time behavior on pathological inputs (ported from tests/pathological.rs)."
import json,subprocess,sys,time

import pytest

from xhtmlmd import to_xhtml

BOUND = 10.0  # generous vs the Rust 5s bound; still catches super-linear blowup
TIGHT = 2.0  # for cases sized so quadratic behavior lands well over this but linear well under

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
    _, el = _timed("![[]()" * 8_000)
    assert el < TIGHT

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

def test_unclosed_inline_math_is_linear():
    html, el = _timed("\\(a " * 10_000)
    assert "<p>(a (a" in html
    assert "math" not in html
    assert el < TIGHT

def test_inline_angle_without_close_is_linear():
    html, el = _timed("<a " * 12_000)
    assert "&lt;a" in html
    assert el < TIGHT

@pytest.mark.slow
def test_deep_marker_chains_do_not_crash():
    """Uncapped container markers must not overflow the native stack.

    Renders in one subprocess so a crash fails the test instead of killing pytest."""
    cases = [
        ("fenced divs", "::: note\n" * 50_000, '<div class="note">'),
        ("markdown html containers", '<div markdown="1">\n' * 50_000, "<div>"),
        ("footnote marker chain", "[^a]: " * 50_000 + "x\n\nref[^a]\n", "fn-a"),
        ("definition marker chain", "term\n" + ": " * 50_000 + "x\n", "<dl>"),
    ]
    code = ("import sys,json\nfrom xhtmlmd import to_xhtml\n"
            "for s in json.load(sys.stdin):\n"
            "    print(json.dumps(to_xhtml(s)), flush=True)\n")
    r = subprocess.run([sys.executable, "-c", code], input=json.dumps([inp for _, inp, _ in cases]),
                       capture_output=True, text=True, timeout=120)
    done = r.stdout.count("\n")
    assert r.returncode == 0, f"crashed on {cases[min(done, len(cases)-1)][0]!r} (rc={r.returncode})"
    for (name, _, frag), out in zip(cases, r.stdout.splitlines()):
        assert frag in json.loads(out), name

def test_numeric_entities_without_semicolon_are_linear():
    html, el = _timed("&#1" * 250_000)
    assert "&amp;#1" in html
    assert el < TIGHT

def test_blank_line_runs_in_lists_are_linear():
    html, el = _timed("- a\n" + "\n" * 12_000 + "b")
    assert "<li>a</li>" in html
    assert el < TIGHT

def test_bare_url_trailing_parens_are_linear():
    html, el = _timed("http://example.com/" + ")" * 15_000)
    assert '<a href="http://example.com/">' in html
    assert el < TIGHT

def test_unmatched_emphasis_closers_are_linear():
    html, el = _timed("a_ " * 45_000)
    assert "<em>" not in html
    assert el < TIGHT
    html, el = _timed("a** " * 35_000)
    assert "<strong>" not in html
    assert el < TIGHT

def test_boundary_runs_without_whitespace_are_linear():
    html, el = _timed("[a]" + "(" * 30_000)
    assert "(((" in html
    assert el < TIGHT

def test_grid_table_many_rows_is_linear():
    html, el = _timed("+---+\n" + "| a |\n+---+\n" * 40_000)
    assert html.count("<td>a</td>") == 40_000
    assert el < TIGHT
