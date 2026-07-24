"""Exporters that lower symbolic MDHTML to finished output, plus the dialect-level machinery
(reference grammar, numbering schemes, raw-payload decoding) shared by all `mdhtml2*` converters."""
import base64, json, re
from html import unescape
from pathlib import Path

from justhtml import Element, Text

from ._html import parse_mdhtml

try: from fastpylight import highlight, highlight_spans
except ImportError: highlight = highlight_spans = None

__all__ = ["SCHEMES", "REFTYPES", "ref_tokens", "ref_variant", "decode_raw", "group_plan", "mustache_kind", "HeadingNums", "Resolver", "to_html", "math_js"]


SCHEMES = dict(decimal={".".join(f"%{j}" for j in range(1, i + 2)): "decimal" for i in range(6)}, legal={
    "%1.": "decimal", "(%2)": "lowerLetter", "(%3)": "lowerRoman", "(%4)": "upperLetter", "(%5)": "upperRoman", "(%6)": "decimal"})

REFTYPES = dict(sec=("Section", "Sections"), fig=("Figure", "Figures"), tbl=("Table", "Tables"))

_VARIANTS = {"page", "text", "leaf", "rel"}


def ref_tokens(val):
    "Validated token set from a `data-ref` attribute value."
    tokens = set((val or "").split())
    if unknown := tokens - _VARIANTS - {"bare"}: raise ValueError(f"unknown data-ref tokens: {sorted(unknown)}")
    if len(variants := tokens & _VARIANTS) > 1: raise ValueError(f"conflicting data-ref tokens: {sorted(variants)}")
    return tokens


def ref_variant(tokens):
    "The rendering variant in a validated token set: 'page', 'text', 'leaf', 'rel', or 'full'."
    return next(iter(tokens & _VARIANTS), "full")


def decode_raw(el):
    "Decoded payload of an MDHTML raw-data script as `(payload, warning)`, one side None."
    payload, encoding = el.to_text(), el.attrs.get("data-encoding")
    try:
        if encoding is None: return payload, None
        if encoding == "html": return unescape(payload), None
        if encoding == "base64": return base64.b64decode(re.sub(r"[ \t\n\f\r]+", "", payload), validate=True).decode(), None
        return None, f"unknown MDHTML raw encoding {encoding!r}; payload dropped"
    except (ValueError, UnicodeDecodeError) as e: return None, f"malformed {encoding} MDHTML raw payload: {e}"


def group_plan(types):
    "Per-item `(separator, prefix?, plural?)` for a `data-refs` group: one pluralized prefix for a same-type group, per-item prefixes for mixed."
    n, mixed = len(types), len(set(types)) > 1
    return [("" if not i else " and " if i == n - 1 else ", ", mixed or not i, not mixed and n > 1) for i in range(n)]


def mustache_kind(body):
    "'section' when a mustache token body opens, closes, or inverts a section (`#`/`/`/`^` sigil), else 'var'."
    return "section" if body.strip()[:1] in "#/^" else "var"


def _letter(n): return chr(ord("a") + (n - 1) % 26) * ((n - 1) // 26 + 1)

def _roman(n):
    out = ""
    for v, s in ((1000, "m"), (900, "cm"), (500, "d"), (400, "cd"), (100, "c"), (90, "xc"), (50, "l"),
        (40, "xl"), (10, "x"), (9, "ix"), (5, "v"), (4, "iv"), (1, "i")):
        while n >= v: out, n = out + s, n - v
    return out

_FMTS = dict(decimal=str, lowerLetter=_letter, upperLetter=lambda n: _letter(n).upper(),
    lowerRoman=_roman, upperRoman=lambda n: _roman(n).upper())


class HeadingNums:
    "Heading numbering per a `{lvlText: numFmt}` scheme (Word semantics): `bump` at each heading, then read its display or full-context number."

    def __init__(self, scheme):
        if isinstance(scheme, str):
            if scheme not in SCHEMES: raise ValueError(f"unknown numbering scheme {scheme!r}")
            scheme = SCHEMES[scheme]
        self.scheme, self.counts = list(scheme.items()), [0] * len(scheme)

    def bump(self, lvl):
        "Advance level `lvl` (0-based), resetting deeper levels; returns the display number, or None beyond the scheme."
        if lvl >= len(self.counts): return None
        self.counts[lvl] += 1
        for i in range(lvl + 1, len(self.counts)): self.counts[i] = 0
        return self.display(lvl)

    def display(self, lvl):
        "The number as shown at a level-`lvl` heading: its lvlText with each %k formatted per level k's numFmt."
        fmt = lambda m: _FMTS[self.scheme[int(m[1]) - 1][1]](self.counts[int(m[1]) - 1])
        return re.sub(r"%(\d)", fmt, self.scheme[lvl][0])

    def full(self, lvl):
        "Word-style full context: ancestor displays concatenated, unless `lvl`'s own lvlText already includes them."
        if lvl == 0 or "%1" in self.scheme[lvl][0]: return self.display(lvl)
        return "".join(self.display(i) for i in range(lvl + 1))


def math_js(fn=None, **opts):
    """JS rendering each MDHTML math carrier in place with KaTeX (the `katex` global must already be loaded):
    a scoped render function guarded against re-rendering, so dynamic pages can re-run it per swapped node.
    `fn` names the emitted function for the caller to wire up; bare `math_js()` renders the whole document
    immediately. `opts` merge into the `katex.render` options (e.g. `minRuleThickness=0.06`)."""
    xtra = "".join(f", {k}: {json.dumps(v)}" for k, v in opts.items())
    body = ("o => { for (const el of o.querySelectorAll('span.math:not([data-mathed]), div.math:not([data-mathed])')) { "
        "el.dataset.mathed = 1; katex.render(el.textContent, el, "
        f"{{displayMode: el.classList.contains('display'), throwOnError: false{xtra}}}); }} }}")
    return f"const {fn} = {body};" if fn else f"({body})(document);"


class Resolver:
    """Cross-reference resolution shared by exporters: a registry of targets (`kinds`, `idtext`,
    `headnums`, `capnums`) and the baked text each reference resolves to."""

    def __init__(self, reftypes=None):
        self.reftypes = REFTYPES | (reftypes or {})
        self.kinds, self.idtext, self.headnums, self.capnums = {}, {}, {}, {}

    def check(self, tgt):
        "Raise for a reference to an unknown target id."
        if tgt not in self.kinds:
            raise ValueError(f"cross-reference target #{tgt} not found (targets are headings, paragraphs, figures, and tables with ids)")

    def core(self, tgt, tokens):
        "A reference's baked text, without any prefix word."
        variant = ref_variant(tokens)
        if variant == "text": return self.idtext[tgt]
        if self.kinds[tgt] == "caption":
            label, n = self.capnums[tgt]
            return str(n) if "bare" in tokens or variant in ("leaf", "rel") else f"{label} {n}"
        if tgt not in self.headnums:
            raise ValueError(f"cross-reference #{tgt} needs a number its target does not have; pass number_headings or use {{ref=text}}")
        display, full = self.headnums[tgt]
        return display if variant == "leaf" else full

    def prefix(self, override, tgt, tokens, plural=False):
        "Prefix text before a reference: `override` text, the type's prefix word, or nothing for bare and caption refs."
        if override: return override + " "
        if "bare" in tokens or self.kinds[tgt] == "caption": return ""
        t = tgt.split("-")[0]
        if t not in self.reftypes: raise ValueError(f"unknown reference type {t!r}; pass reftypes= to define its prefix")
        return self.reftypes[t][plural] + " "



class Html(str):
    "Exported HTML, with the export's `warnings` attached."

    def __new__(cls, s, warnings):
        self = super().__new__(cls, s)
        self.warnings = warnings
        return self

    def __getnewargs__(self): return (str(self), self.warnings)


_HEADS = {"h1", "h2", "h3", "h4", "h5", "h6"}
_RAW_TYPE = "application/vnd.mdhtml.raw"


def _els(el): return [c for c in el.children if isinstance(c, Element)]

def _walk(el):
    yield el
    for c in _els(el): yield from _walk(c)

def _text(el): return " ".join(el.to_text().split())

def _mk(name, attrs=None, text=None):
    el = Element(name, attrs or {}, None)
    if text is not None: el.append_child(Text(text))
    return el

def _set_children(el, nodes):
    for c in list(el.children): el.remove_child(c)
    for c in nodes: el.append_child(c)

def _insert_first(el, node): el.insert_before(node, el.children[0] if el.children else None)


class _Exporter(Resolver):
    def __init__(self, reftypes, number_headings, hl, toc, refs="resolve", id_prefix="", fn_salt="", hl_lang=None, code_wrap=None):
        super().__init__(reftypes)
        self.number_headings, self.hl, self.toc, self.refs = number_headings, hl, toc, refs
        self.id_prefix, self.fn_salt, self.hl_lang, self.code_wrap = id_prefix, fn_salt, hl_lang, code_wrap
        self.warnings = []

    def run(self, root):
        els = [e for c in root.children if isinstance(c, Element) for e in _walk(c)]
        self.heads = [e for e in els if e.name in _HEADS]
        for e in els:
            if not (i := e.attrs.get("id")): continue
            self.idtext[i] = _text(e)
            if e.name in ("figure", "table"): self.kinds[i] = "caption"
            elif e.name in _HEADS or e.name == "p": self.kinds[i] = "block"
        groups = [e for e in els if "data-refs" in e.attrs]
        grouped = {id(a) for g in groups for a in _els(g) if a.name == "a"}
        singles = [e for e in els if e.name == "a" and "data-ref" in e.attrs and id(e) not in grouped]
        refs = []
        if self.refs == "resolve":
            refs = [(a, self._parse_ref(a)) for g in groups for a in _els(g) if a.name == "a"]
            refs += [(a, self._parse_ref(a)) for a in singles]
            self._number_headings(refs)
            self._number_captions([e for e in els if e.name in ("figure", "table")])
        if self.refs == "resolve":
            for g in groups: self._lower_group(g)
            for a in singles:
                tgt, tokens = self._parse_ref(a)
                _set_children(a, [Text(self.prefix(a.to_text().strip(), tgt, tokens) + self.core(tgt, tokens))])
                del a.attrs["data-ref"]
        else:
            for g in groups: self._lower_group_ids(g)
            for a in singles: self._bake_id(a)
        self._prefix_ids(els)
        self._raw([e for e in els if e.name == "script" and e.attrs.get("type") == _RAW_TYPE])
        for t in els:
            if t.name == "table" and "colwidths" in t.attrs: self._colgroup(t)
        if (self.hl and highlight_spans) or self.hl_lang or self.code_wrap:
            for pre in els:
                if pre.name == "pre": self._hl(pre)
        if self.toc: _insert_first(root, self._toc_nav())

    def _parse_ref(self, a):
        tgt = (a.attrs.get("href") or "#")[1:]
        tokens = ref_tokens(a.attrs.get("data-ref"))
        self.check(tgt)
        return tgt, tokens

    def _number_headings(self, refs):
        "Number the headings when a scheme is given, or when a ref needs a heading number (auto 'decimal')."
        needed = any(self.kinds[tgt] == "block" and ref_variant(tokens) != "text" for _, (tgt, tokens) in refs)
        if not (self.number_headings or needed): return
        nums = HeadingNums(self.number_headings or "decimal")
        for el in self.heads:
            lvl = int(el.name[1]) - 1
            if (d := nums.bump(lvl)) is None: continue
            _insert_first(el, Text(" "))
            _insert_first(el, _mk("span", {"class": "heading-number"}, d))
            if i := el.attrs.get("id"): self.headnums[i] = (d, nums.full(lvl))

    def _number_captions(self, els):
        "Number figures and tables that have a caption or an id, baking 'Label N' into the caption."
        counts = {}
        for el in els:
            fig = el.name == "figure"
            capel = next((c for c in _els(el) if c.name == ("figcaption" if fig else "caption")), None)
            if capel is None and "id" not in el.attrs: continue
            label = self.reftypes["fig" if fig else "tbl"][0]
            n = counts[label] = counts.get(label, 0) + 1
            if i := el.attrs.get("id"): self.capnums[i] = (label, n)
            span = _mk("span", {"class": "caption-label"}, f"{label} {n}")
            if capel is None:
                capel = _mk("figcaption" if fig else "caption")
                capel.append_child(span)
                if fig: el.append_child(capel)
                else: _insert_first(el, capel)
            else:
                _insert_first(capel, Text(": "))
                _insert_first(capel, span)

    def _lower_group(self, span):
        anchors = [a for a in _els(span) if a.name == "a"]
        out = []
        types = [(a.attrs.get("href") or "#")[1:].split("-")[0] for a in anchors]
        for (sep, pre, plural), a in zip(group_plan(types), anchors):
            if sep: out.append(Text(sep))
            tgt, tokens = self._parse_ref(a)
            if pre and (p := self.prefix(a.to_text().strip(), tgt, tokens, plural)): out.append(Text(p))
            _set_children(a, [Text(self.core(tgt, tokens))])
            del a.attrs["data-ref"]
            out.append(a)
        del span.attrs["data-refs"]
        _set_children(span, out)

    def _bake_id(self, a):
        "Bake one ref anchor as an id-text link (`refs='ids'`): author text kept as a prefix, `xref` class for styling."
        tgt = (a.attrs.get("href") or "#")[1:]
        text = a.to_text().strip()
        _set_children(a, [Text(f"{text} {tgt}" if text else tgt)])
        a.attrs["href"] = f"#{self.id_prefix}{tgt}"
        a.attrs["class"] = "xref"
        del a.attrs["data-ref"]

    def _lower_group_ids(self, span):
        anchors = [a for a in _els(span) if a.name == "a"]
        out = []
        for (sep, _, _), a in zip(group_plan([""] * len(anchors)), anchors):
            if sep: out.append(Text(sep))
            self._bake_id(a)
            out.append(a)
        del span.attrs["data-refs"]
        _set_children(span, out)

    def _prefix_ids(self, els):
        "Namespace ids under `id_prefix`: every element id (original kept in `data-id`), plus links to in-fragment ids."
        if not (self.id_prefix or self.fn_salt): return
        ids = {i for e in els if (i := e.attrs.get("id"))}
        pfx = lambda i: self.id_prefix + (self.fn_salt if i.startswith(("fn-", "fnref-")) else "")
        for e in els:
            if i := e.attrs.get("id"): e.attrs.update({"id": pfx(i) + i, "data-id": i})
            if e.name == "a" and (h := e.attrs.get("href") or "").startswith("#") and h[1:] in ids:
                e.attrs["href"] = f"#{pfx(h[1:])}{h[1:]}"

    def _raw(self, scripts):
        for el in scripts:
            payload, warn = None, None
            if el.attrs.get("data-format") == "html": payload, warn = decode_raw(el)
            if warn: self.warnings.append(warn)
            if payload is None: el.parent.remove_child(el)
            else: el.parent.replace_child(parse_mdhtml(payload), el)

    def _colgroup(self, el):
        "Lower a `colwidths` attribute to a colgroup; `fr` values share the width remaining after fixed lengths."
        toks = el.attrs.pop("colwidths").split()
        fixed = [t for t in toks if not t.endswith("fr")]
        tot = sum(float(t[:-2]) for t in toks if t.endswith("fr"))
        cg = _mk("colgroup")
        for t in toks:
            if t.endswith("fr"):
                share = float(t[:-2]) / tot
                w = f"calc((100% - {' - '.join(fixed)}) * {share:g})" if fixed else f"{share * 100:g}%"
            else: w = t
            cg.append_child(_mk("col", {"style": f"width:{w}"}))
        style = el.attrs.get("style")
        el.attrs["style"] = (style.rstrip("; ") + ";" if style else "") + "table-layout:fixed;width:100%"
        kids = el.children
        pos = 1 if kids and isinstance(kids[0], Element) and kids[0].name == "caption" else 0
        el.insert_before(cg, kids[pos] if pos < len(kids) else None)

    def _hl(self, pre):
        code = next((c for c in _els(pre) if c.name == "code"), None)
        if code is None: return
        lang = next((c.removeprefix("language-") for c in (code.attrs.get("class") or "").split()
            if c.startswith("language-")), None)
        text = code.to_text()
        if self.hl_lang and (new := self.hl_lang(text, lang)) != lang:
            lang = new
            if lang: code.attrs["class"] = f"language-{lang}"
        if self.hl and highlight_spans and lang is not None:
            try:
                if self.hl == "spans":
                    frag = parse_mdhtml(highlight_spans(text, lang))
                    _set_children(code, list(frag.children[0].children[0].children))
                else:
                    frag = parse_mdhtml(highlight(text, lang))
                    hlc = _mk("hl-code", {"toks": frag.children[0].attrs.get("toks")})
                    pre.parent.replace_child(hlc, pre)
                    hlc.append_child(pre)
                    pre = hlc
            except ValueError: pass
        if self.code_wrap and (repl := self.code_wrap(pre.to_html(pretty=False), lang, text)) is not None:
            pre.parent.replace_child(parse_mdhtml(repl), pre)

    def _toc_nav(self):
        nav = _mk("nav", {"class": "toc"})
        stack, levels = [_mk("ol")], [None]
        nav.append_child(stack[0])
        for el in self.heads:
            lvl = int(el.name[1])
            if levels[-1] is None: levels[-1] = lvl
            while lvl < levels[-1] and len(stack) > 1:
                stack.pop()
                levels.pop()
            if lvl > levels[-1]:
                sub = _mk("ol")
                host = stack[-1].children[-1] if stack[-1].children else stack[-1]
                host.append_child(sub)
                stack.append(sub)
                levels.append(lvl)
            li = _mk("li")
            if i := el.attrs.get("id"): li.append_child(_mk("a", {"href": f"#{i}"}, _text(el)))
            else: li.append_child(Text(_text(el)))
            stack[-1].append_child(li)
        return nav


def to_html(src, dest=None, reftypes: dict | None = None, number_headings=None, hl: str | None = "spans",
    toc: bool = False, refs: str = "resolve", id_prefix: str = "", fn_salt: str = "", hl_lang=None, code_wrap=None) -> Html:
    """Lower MDHTML (a string or DocumentFragment; never mutated) to finished HTML: cross-references
    baked as links, headings and captions numbered, `{=html}` raw data spliced, `colwidths` lowered,
    and code highlighted. `refs='ids'` instead bakes each reference as a working link showing its
    target id (class `xref`), with no registry, numbering, or failure modes - for live-preview
    contexts where targets may sit outside the fragment. `id_prefix` namespaces the output's ids:
    every element id is prefixed (original kept in `data-id`), along with ref hrefs and links to
    in-fragment ids; links to outside ids are untouched. `fn_salt` is an extra prefix for footnote
    ids only (`fn-*`/`fnref-*`), so fragments sharing one `id_prefix` keep their footnote pairs
    distinct. Per code block, `hl_lang(text, lang)` may
    return a corrected language (`lang` is None for a bare fence), and `code_wrap(html, lang, text)`
    may return replacement markup for the highlighted block (None keeps it; `text` is unescaped).
    Returns an `Html` str carrying `.warnings`; `dest` also writes it to a file."""
    if refs not in ("resolve", "ids"): raise ValueError(f"unknown refs mode {refs!r}")
    if not isinstance(src, str): src = src.to_html(pretty=False)
    root = parse_mdhtml(src)
    ex = _Exporter(reftypes, number_headings, hl, toc, refs, id_prefix, fn_salt, hl_lang, code_wrap)
    ex.run(root)
    res = Html(root.to_html(pretty=False), ex.warnings)
    if dest is not None: Path(dest).write_text(res, encoding="utf-8")
    return res
