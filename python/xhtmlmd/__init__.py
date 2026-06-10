from ._native import to_xhtml as _to_xhtml

__all__ = ["to_xhtml", "render"]


def to_xhtml(markdown: str, *, math: str = "brackets", tagfilter: bool = False, max_inline_depth: int | None = None,
    max_block_depth: int | None = None, max_link_paren_depth: int | None = None) -> str:
    "Render Markdown to an XHTML fragment."
    return _to_xhtml(markdown, math=math, tagfilter=tagfilter, max_inline_depth=max_inline_depth, max_block_depth=max_block_depth,
        max_link_paren_depth=max_link_paren_depth)


render = to_xhtml
