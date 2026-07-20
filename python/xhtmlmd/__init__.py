from ._native import to_xhtml as _to_xhtml, blocks as _blocks, edit_nodes as _edit_nodes

__all__ = ["to_xhtml", "render", "blocks", "rewrite", "sample_md"]


def to_xhtml(markdown: str, *, math: str = "brackets", tagfilter: bool = False, balance: bool = False,
    auto_ids: bool = True, smart: bool = False, callbacks: dict | None = None,
    max_inline_depth: int | None = None, max_block_depth: int | None = None, max_link_paren_depth: int | None = None) -> str:
    "Render Markdown to an XHTML fragment."
    return _to_xhtml(markdown, math=math, tagfilter=tagfilter, balance=balance, auto_ids=auto_ids, smart=smart,
        callbacks=callbacks, max_inline_depth=max_inline_depth,
        max_block_depth=max_block_depth, max_link_paren_depth=max_link_paren_depth)


render = to_xhtml


def sample_md() -> str:
    "The packaged feature-sample Markdown document, exercising the full dialect."
    from importlib.resources import files
    return (files("xhtmlmd") / "sample.md").read_text(encoding="utf-8")


def blocks(markdown: str, *, math: str = "brackets") -> list[dict]:
    "Top-level block source spans: dicts with `type` and half-open 0-based `start`/`end` line indices into `markdown.split('\\n')`; code and math blocks also carry `text` (and `info`/`lang` for fences)."
    return _blocks(markdown, math=math)


def rewrite(markdown: str, callbacks: dict, *, math: str = "brackets") -> str:
    "Rewrite recognized Markdown constructs while preserving all other source text."
    normalized, offsets = _normalize_offsets(markdown)
    edits = []
    for raw in _edit_nodes(normalized, math=math):
        norm_start, norm_end = raw["start"], raw["end"]
        start, end = offsets[norm_start], offsets[norm_end]
        internal = {k: raw.pop(k) for k in tuple(raw) if k.startswith("_")}
        raw.update(source=markdown[start:end], start=start, end=end)
        callback = callbacks.get(raw["type"])
        if callback is None: continue
        replacement = callback(raw)
        if replacement is None: continue
        if isinstance(replacement, str):
            edits.append((start, end, replacement))
            continue
        if not isinstance(replacement, dict): raise TypeError(f"{raw['type']} callback must return None, str, or dict")
        allowed = {"url"} if raw["type"] == "image" else {"tex"}
        unknown = replacement.keys() - allowed
        if unknown: raise ValueError(f"unknown {raw['type'].replace('_inline', '')} replacement field: {sorted(unknown)[0]}")
        if any(not isinstance(value, str) for value in replacement.values()):
            raise TypeError(f"{raw['type']} replacement fields must be strings")
        if raw["type"] == "image" and "url" in replacement:
            edits.append((offsets[internal["_url_start"]], offsets[internal["_url_end"]], replacement["url"]))
        if raw["type"] == "math_inline" and "tex" in replacement:
            n = len(raw["delimiter"])
            edits.append((offsets[norm_start + n], offsets[norm_end - n], replacement["tex"]))
    for start, end, replacement in reversed(edits): markdown = markdown[:start] + replacement + markdown[end:]
    return markdown


def _normalize_offsets(src: str) -> tuple[str, list[int]]:
    out, offsets = [], [0]
    i = 0
    while i < len(src):
        start = i
        if src[i] == "\r":
            n = 2 if i + 1 < len(src) and src[i + 1] == "\n" else 1
            ch = "\n"
            i += n
        else:
            ch = src[i]
            i += 1
        out.append(ch)
        offsets.extend([start] * (len(ch.encode()) - 1))
        offsets.append(i)
    return "".join(out), offsets
