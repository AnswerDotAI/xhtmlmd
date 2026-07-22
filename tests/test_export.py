import pytest

from mdhtml import math_js, parse_mdhtml, to_html, to_md, to_mdhtml

REFS_MD = """# Payment {#sec-pay}

## Late fees {#sec-late}

See [@sec-pay], [-@sec-late], [Clause @sec-late], [@sec-pay; @sec-late], [@sec-late]{ref=leaf},
[-@sec-late]{ref=text}, and page [-@sec-late]{ref=page}.
"""

def test_refs_and_heading_numbering():
    h = to_html(to_mdhtml(REFS_MD), number_headings='legal')
    assert '<span class="heading-number">1.</span> Payment' in h
    assert '<span class="heading-number">(a)</span> Late fees' in h
    assert '<a href="#sec-pay">Section 1.</a>' in h
    assert '<a href="#sec-late">1.(a)</a>' in h                    # bare: no prefix word
    assert '<a href="#sec-late">Clause 1.(a)</a>' in h             # override text
    assert 'Sections <a href="#sec-pay">1.</a> and <a href="#sec-late">1.(a)</a>' in h
    assert '<a href="#sec-late">Section (a)</a>' in h              # leaf
    assert '<a href="#sec-late">Late fees</a>' in h                # text
    assert 'page <a href="#sec-late">1.(a)</a>' in h               # page degrades to full
    assert 'data-ref' not in h
    assert h.warnings == []
    d = to_html(to_mdhtml('# One {#sec-a}\n\n## Two {#sec-b}\n\nSee [@sec-b].'), number_headings='decimal')
    assert '<span class="heading-number">1.1</span> Two' in d
    assert '<a href="#sec-b">Section 1.1</a>' in d


def test_ref_errors():
    with pytest.raises(ValueError, match='not found'): to_html(to_mdhtml('See [@sec-x].'))
    auto = to_html(to_mdhtml('# A {#sec-a}\n\nSee [@sec-a].'))       # refs trigger auto decimal numbering
    assert '<span class="heading-number">1</span> A' in auto and '<a href="#sec-a">Section 1</a>' in auto
    assert 'heading-number' not in to_html(to_mdhtml('# A {#sec-a}\n\nText.'))   # no numeric ref: no numbering
    md = '# A {#exh-a}\n\nSee [@exh-a].'
    with pytest.raises(ValueError, match='reftypes'): to_html(to_mdhtml(md), number_headings='legal')
    h = to_html(to_mdhtml(md), number_headings='legal', reftypes=dict(exh=('Exhibit', 'Exhibits')))
    assert '<a href="#exh-a">Exhibit 1.</a>' in h
    with pytest.raises(ValueError, match='data-ref'):
        to_html('<p id="x">t</p><p><a data-ref="zap" href="#x"></a></p>')


def test_captions_and_caption_refs():
    md = ("![A diagram](d.png){#fig-d}\n\n![Second](e.png){#fig-e}\n\n"
        "| A |\n|---|\n| 1 |\n: Stages {#tbl-s}\n\nSee [@fig-d], [@fig-e; @tbl-s], and [-@tbl-s].")
    h = to_html(to_mdhtml(md, implicit_figures=True))
    assert '<figcaption><span class="caption-label">Figure 1</span>: A diagram</figcaption>' in h
    assert '<caption><span class="caption-label">Table 1</span>: Stages</caption>' in h
    assert '<a href="#fig-d">Figure 1</a>' in h
    assert '<a href="#fig-e">Figure 2</a> and <a href="#tbl-s">Table 1</a>' in h
    assert '<a href="#tbl-s">1</a>' in h                           # bare caption ref: number only
    bare = to_html('<figure id="fig-x"><img src="x.png" alt=""></figure>')
    assert '<figcaption><span class="caption-label">Figure 1</span></figcaption>' in bare
    plain = to_html('<figure><img src="x.png" alt=""><figcaption>Cap</figcaption></figure>')
    assert '<span class="caption-label">Figure 1</span>: Cap' in plain


def test_raw_html():
    h = to_html(to_mdhtml('Before\n\n```{=html}\n<aside>Hi</aside>\n```\n\n```{=docx}\n<w:p/>\n```\n'))
    assert '<aside>Hi</aside>' in h
    assert 'script' not in h and 'w:p' not in h
    enc = '<script type="application/vnd.mdhtml.raw" data-format="html" data-encoding="html">&lt;b&gt;x&lt;/b&gt;</script>'
    assert '<b>x</b>' in to_html(enc)
    bad = to_html('<script type="application/vnd.mdhtml.raw" data-format="html" data-encoding="rot13">x</script>')
    assert 'rot13' in bad.warnings[0]
    inline = to_html(to_mdhtml('An `<u>x</u>`{=html} inline.'))
    assert '<p>An <u>x</u> inline.</p>' in inline


def test_colwidths():
    h = to_html(to_mdhtml('| A | B | C |\n|---|---|---|\n| 1 | 2 | 3 |\n: Cap {colwidths="10em 1fr 1fr"}'))
    assert '<colgroup>' in h and h.count('<col ') == 3
    assert '10em' in h and 'calc' in h
    assert 'colwidths' not in h


def test_hl_modes():
    mdh = to_mdhtml('``` python {.numberLines}\nx = 1\n```')
    h = to_html(mdh)
    assert '<span class="hl-' in h and 'numberLines' in h
    assert '<span class="hl-' not in to_html(mdh, hl=None)
    api = to_html(mdh, hl='api')
    assert '<hl-code toks=' in api and 'x = 1' in api


def test_toc():
    h = to_html(to_mdhtml('# One {#sec-a}\n\nText.\n\n## Two {#sec-b}\n\n# Three'), toc=True)
    assert '<nav class="toc">' in h
    assert '<a href="#sec-a">One</a>' in h and '<a href="#sec-b">Two</a>' in h
    assert 'Three' in h.split('</nav>')[0]                         # id-less heading still listed


def test_api_shape(tmp_path):
    frag = parse_mdhtml('<p id="x">Hi</p><p><a data-ref="bare text" href="#x"></a></p>')
    before = frag.to_html(pretty=False)
    h = to_html(frag)
    assert frag.to_html(pretty=False) == before                    # input fragment not mutated
    assert '<a href="#x">Hi</a>' in h
    out = tmp_path/'o.html'
    to_html('<p>Hi</p>', dest=out)
    assert out.read_text() == '<p>Hi</p>'
    assert 'katex' in math_js()

def test_math_js():
    js = math_js(fn='render', minRuleThickness=0.06)
    assert js.startswith('const render = ') and 'minRuleThickness: 0.06' in js
    assert 'data-mathed' in js and 'katex.render' in js
    bare = math_js()
    assert bare.endswith('(document);') and 'katex.render' in bare

def test_result_types_copy():
    import copy, pickle
    for r in (to_html('<p>See <a data-ref="" href="#nope"></a></p>', strict_refs=False), to_md('# A {#sec-a}\n')):
        for c in (copy.deepcopy(r), pickle.loads(pickle.dumps(r))): assert (c, type(c), c.warnings) == (r, type(r), r.warnings)






def test_code_hooks():
    src = to_mdhtml('```\n%%sql\nSELECT 1\n```\n\n```mermaid\ngraph TD\n```\n')
    def wrap(html, lang, text):
        if lang == 'mermaid': return f'<pre class="mermaid">{text}</pre>'
        return f'<div class="copy-wrap">{html}</div>'
    h = to_html(src, code_wrap=wrap,
        hl_lang=lambda text, lang: text.split('\n')[0].removeprefix('%%') if text.startswith('%%') else lang)
    assert 'language-sql' in h and '<span class="hl-keyword">SELECT</span>' in h   # remapped, then highlighted
    assert '<div class="copy-wrap"><pre>' in h
    assert '<pre class="mermaid">graph TD</pre>' in h and 'language-mermaid' not in h

def test_strict_refs_off():
    src = to_mdhtml('# A {#zap-a}\n\nSee [@sec-nope], [@zap-a], and [@fig-e; @sec-nope].\n\n![E](e.png){#fig-e}\n',
        implicit_figures=True)
    h = to_html(src, strict_refs=False)
    assert '#sec-nope' in h and '#zap-a' in h            # failing refs degrade to plain text
    assert 'Figure 1' in h                               # healthy refs in the same doc still resolve
    assert len(h.warnings) == 3
    with pytest.raises(ValueError): to_html(src)         # strict stays the default


def test_to_md_refs_and_numbering():
    out = to_md(REFS_MD, number_headings='legal')
    assert '# 1. Payment\n' in out and '## (a) Late fees\n' in out
    assert '{#sec-pay}' not in out
    assert ('See Section 1., 1.(a), Clause 1.(a), Sections 1. and 1.(a), Section (a),\n'
        'Late fees, and page 1.(a).') in out
    auto = to_md('# A {#sec-a}\n\nSee [@sec-a].')
    assert '# 1 A\n' in auto and 'See Section 1.' in auto
    assert to_md('# A {#sec-a}\n\nText only.\n') == '# A\n\nText only.\n'   # strip only; rest byte-identical
    with pytest.raises(ValueError, match='not found'): to_md('See [@sec-x].')


def test_to_md_nested_containers():
    md = ('# Top {#sec-top}\n\n<div markdown="1">\n\n## Inner {#sec-in}\n\nBody.\n\n</div>\n\n'
        '> ## Quoted {#sec-q}\n\nSee [@sec-top], [@sec-in], and [-@sec-q]{ref=text}.\n')
    out = to_md(md)
    assert '# 1 Top\n' in out and '## 1.1 Inner\n' in out
    assert '{#sec-in}' not in out
    assert 'See Section 1, Section 1.1, and Quoted.' in out
    assert '{#sec-q}' in out                      # marker containers pass through unrewritten
    assert any('sec-q' in w or 'line 11' in w for w in out.warnings)
    from mdhtml import sample_md
    smp = to_md(sample_md(), implicit_figures=True)      # the full feature sample lowers cleanly
    assert 'per Section ' in smp                         # refs in the markdown="1" container resolve
    assert smp.count('{#sec-payment}') == 1              # real heading stripped; fenced example untouched


def test_to_md_captions_and_figures():
    md = ('| A |\n|---|\n| 1 |\n: Stages {#tbl-s}\n\n![A diagram](d.png){#fig-d}\n\n'
        'See [@tbl-s] and [-@fig-d].')
    out = to_md(md, implicit_figures=True)
    assert '| 1 |\n\nTable 1: Stages\n' in out and '{#tbl-s}' not in out
    assert '![A diagram](d.png)\n\nFigure 1: A diagram\n' in out
    assert 'See Table 1 and 1.' in out


def test_to_md_strip_and_raw():
    md = ('A [word]{.hl} and [link](u){.x} and `c`{.y}.\n\n{: .note}\nPara with IAL.\n\n'
        '{:ald: .cls}\n\n*[HTML]: HyperText\n\n::: warn\nInner *md*.\n:::\n\n'
        '```{=md}\nRaw *stays*.\n```\n\n```{=docx}\n<w:p/>\n```\n')
    out = to_md(md)
    assert 'A word and [link](u) and `c`.' in out
    assert '{: .note}' not in out and 'Para with IAL.' in out
    assert '{:ald:' not in out and '*[HTML]' not in out
    assert ': warn' not in out and 'Inner *md*.' in out and ':::' not in out
    assert 'Raw *stays*.' in out and '{=md}' not in out and 'w:p' not in out
    assert out.warnings == []


def test_to_md_passthrough():
    md = ('Text[^1] with $x$ math and | pipes |.\n\n[^1]: A note.\n\n'
        '| A | B |\n|---|---|\n| 1 | 2 |\n\n- [x] done\n\n[ref link][r]\n\n[r]: /url\n')
    assert to_md(md, math='dollars') == md   # nothing to lower: byte-identical
