"Spec conformance: render fixtures and compare normalized HTML trees (ported from tests/conformance.rs)."
from pathlib import Path

import pytest
from bs4 import BeautifulSoup, CData, Comment, Doctype, NavigableString, ProcessingInstruction, Tag

from xhtmlmd import to_xhtml

SOURCE = Path(__file__).parent / "source"
FENCE = "`" * 32
PHP_EXTRA_ACTIVE = [
    "Headers with attributes", "Tables", "Link & Image Attributes", "Definition Lists",
    "Backtick Fenced Code Blocks", "Tilde Fenced Code Blocks",
    "Backtick Fenced Code Blocks Special Cases", "Tilde Fenced Code Blocks Special Cases",
    "Abbr", "Emphasis", "Footnotes"]

BOOL_ATTRS = {"allowfullscreen", "async", "autofocus", "autoplay", "checked", "controls", "default",
    "defer", "disabled", "formnovalidate", "hidden", "ismap", "loop", "multiple", "muted",
    "novalidate", "open", "readonly", "required", "reversed", "selected"}
BLOCK_TAGS = {"address", "article", "aside", "blockquote", "body", "br", "button", "canvas", "caption",
    "col", "colgroup", "dd", "details", "div", "dl", "dt", "embed", "fieldset", "figcaption", "figure",
    "footer", "form", "h1", "h2", "h3", "h4", "h5", "h6", "header", "hgroup", "hr", "iframe", "li", "map",
    "object", "ol", "output", "p", "pre", "progress", "section", "table", "tbody", "td", "textarea",
    "tfoot", "th", "thead", "tr", "ul", "video"}


def _split_inclusive_nl(s):
    parts = s.split("\n")
    out = [p + "\n" for p in parts[:-1]]
    if parts[-1]: out.append(parts[-1])
    return out

def _heading_text(line):
    n = len(line) - len(line.lstrip("#"))
    if n > 0 and line[n:n+1] == " ": return line[n+1:].strip()
    return None

def parse_cmark_examples(name, src):
    "Parse a cmark-gfm style spec file into (name, example, section, markdown, html) cases."
    state, section, md, html, exts, example, out = "text", "", "", "", [], 0, []
    for line in _split_inclusive_nl(src):
        t = line.rstrip("\n").rstrip("\r")
        if state == "text":
            if t.startswith(f"{FENCE} example"):
                state, exts, md, html = "md", t[len(f"{FENCE} example"):].split(), "", ""
            else:
                h = _heading_text(t)
                if h is not None: section = h
        elif state == "md":
            if t == ".": state = "html"
            else: md += line
        elif state == "html":
            if t == FENCE:
                example += 1
                if "disabled" not in exts:
                    out.append((name, example, section, md.replace("→", "\t"), html.replace("→", "\t")))
                state = "text"
            else: html += line
    return out

def parse_mdtest_examples(name, rel_dir, active):
    d = SOURCE / rel_dir
    out = []
    for i, stem in enumerate(active):
        inp = d / f"{stem}.text"
        exp = d / f"{stem}.xhtml"
        if not exp.exists(): exp = d / f"{stem}.html"
        out.append((name, i+1, f"PHP Markdown Extra: {stem}", inp.read_text(), exp.read_text()))
    return out

def all_cases():
    cases = (
        parse_cmark_examples("spec.txt", (SOURCE/"cmark-gfm"/"spec.txt").read_text())
        + parse_cmark_examples("extensions.txt", (SOURCE/"cmark-gfm"/"extensions.txt").read_text())
        + parse_cmark_examples("mf.txt", (SOURCE/"mf.txt").read_text())
        + parse_cmark_examples("kramdown.txt", (SOURCE/"kramdown.txt").read_text())
        + parse_mdtest_examples("php-markdown-extra.mdtest", "php-markdown-extra.mdtest", PHP_EXTRA_ACTIVE))
    return cases


def _collapse_ws(s):
    out, in_space = [], False
    for ch in s:
        if ch.isspace():
            if not in_space: out.append(" "); in_space = True
        else: out.append(ch); in_space = False
    return "".join(out)

def _norm_attr_value(name, value):
    if name in BOOL_ATTRS and (value == "" or value.lower() == name.lower()): return ""
    return value

def _norm_text_edges(children, parent_is_block, in_pre):
    if in_pre: return
    is_block = [c[0] == "el" and c[1] in BLOCK_TAGS for c in children]
    last = len(children) - 1
    for i, c in enumerate(children):
        if c[0] != "text": continue
        text = c[1]
        if (parent_is_block and i == 0) or (i >= 1 and is_block[i-1]): text = text.lstrip()
        if (parent_is_block and i == last) or (i+1 <= last and is_block[i+1]): text = text.rstrip()
        c[1] = text
    children[:] = [c for c in children if not (c[0] == "text" and c[1] == "")]

def _norm_node(node, in_pre):
    if isinstance(node, (Doctype,)): return ["doctype", str(node)]
    if isinstance(node, Comment): return ["comment", str(node)]
    if isinstance(node, (ProcessingInstruction, CData)): return None
    if isinstance(node, NavigableString):
        text = str(node) if in_pre else _collapse_ws(str(node))
        return ["text", text] if text else None
    if isinstance(node, Tag):
        tag = node.name
        nxt = in_pre or tag == "pre"
        attrs = []
        for k, v in node.attrs.items():
            val = " ".join(v) if isinstance(v, list) else ("" if v is None else str(v))
            attrs.append((k, _norm_attr_value(k, val)))
        attrs.sort()
        children = [n for n in (_norm_node(c, nxt) for c in node.children) if n is not None]
        _norm_text_edges(children, tag in BLOCK_TAGS, nxt)
        return ["el", tag, attrs, children]
    return None

def normalize_html(s):
    soup = BeautifulSoup(s, "lxml")
    roots = soup.body.contents if soup.body is not None else soup.contents
    children = [n for n in (_norm_node(c, False) for c in roots) if n is not None]
    _norm_text_edges(children, True, False)
    return children


_CASES = all_cases()

@pytest.mark.parametrize("name,example,section,md,html", _CASES,
    ids=[f"{c[0]}:{c[1]}:{c[2]}" for c in _CASES])
def test_conformance(name, example, section, md, html):
    actual = to_xhtml(md, math="off")
    assert normalize_html(html) == normalize_html(actual)
