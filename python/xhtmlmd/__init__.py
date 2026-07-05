from ._native import to_xhtml as _to_xhtml, blocks as _blocks

__all__ = ["to_xhtml", "render", "blocks"]


def to_xhtml(markdown: str, *, math: str = "brackets", tagfilter: bool = False, balance: bool = False, underline: bool = False,
    callbacks: dict | None = None,
    max_inline_depth: int | None = None, max_block_depth: int | None = None, max_link_paren_depth: int | None = None) -> str:
    "Render Markdown to an XHTML fragment."
    return _to_xhtml(markdown, math=math, tagfilter=tagfilter, balance=balance, underline=underline, callbacks=callbacks,
        max_inline_depth=max_inline_depth, max_block_depth=max_block_depth, max_link_paren_depth=max_link_paren_depth)


render = to_xhtml


def blocks(markdown: str, *, math: str = "brackets") -> list[dict]:
    "Top-level block source spans: dicts with `type` and half-open 0-based `start`/`end` line indices into `markdown.split('\\n')`; code and math blocks also carry `text` (and `info`/`lang` for fences)."
    return _blocks(markdown, math=math)
