"JustHTML integration, including temporary fixes from justhtml#68, justhtml#69, justhtml#70, and justhtml#71."

from justhtml import DocumentFragment, JustHTML, Node
from justhtml.parser import FragmentContext
from justhtml.parser.engine import ParseEngine, _CountingStack
import justhtml.serializer as _serializer_api, justhtml.serializer.html as _serializer
_serializer._LITERAL_TEXT_SERIALIZATION_ELEMENTS = frozenset({"iframe", "noembed", "noframes", "plaintext", "script", "style", "xmp"})

_BOOLEAN_ATTRIBUTES = {}
for _spec in """*:inert itemscope
audio:autoplay controls loop muted
button:autofocus disabled formnovalidate
details:open
dialog:open
fieldset:disabled
form:novalidate
iframe:allowfullscreen credentialless
img:ismap
input:autofocus checked disabled formnovalidate multiple readonly required
link:disabled
ol:reversed
optgroup:disabled
option:disabled selected
script:async defer nomodule
select:autofocus disabled multiple required
template:shadowrootclonable shadowrootdelegatesfocus shadowrootserializable
textarea:autofocus disabled readonly required
track:default
video:autoplay controls loop muted playsinline""".splitlines():
    _element,_attrs = _spec.split(":")
    _BOOLEAN_ATTRIBUTES[_element] = frozenset(_attrs.split())


def _serialize_start_tag(name, attrs, *, quote_attr_values=True, minimize_boolean_attributes=True, quote_char=None,
    use_trailing_solidus=False, is_void=False):
    name = _serializer._validate_serializable_tag_name(name)
    if not attrs: return f"<{name} />" if use_trailing_solidus and is_void else f"<{name}>"
    parts = ["<", name]
    for raw_key,value in attrs.items():
        key = _serializer._validate_serializable_attr_name(raw_key)
        if minimize_boolean_attributes:
            if value is None or value == "":
                parts.extend((" ", key))
                continue
            key_lower = key.lower()
            boolean_attrs = _BOOLEAN_ATTRIBUTES.get(name.lower(), ())
            if (key_lower in _BOOLEAN_ATTRIBUTES["*"] or key_lower in boolean_attrs) and value.lower() == key_lower:
                parts.extend((" ", key))
                continue
        if value is None or value == "":
            parts.extend((" ", key, '=""'))
            continue
        if not quote_attr_values and _serializer._can_unquote_attr_value(value):
            parts.extend((" ", key, "=", value.replace("&", "&amp;").replace("<", "&lt;")))
            continue
        quote = _serializer._choose_attr_quote(value, quote_char)
        parts.extend((" ", key, "=", quote, _serializer._escape_attr_value(value, quote), quote))
    if use_trailing_solidus and is_void: parts.append(" />")
    else: parts.append(">")
    return "".join(parts)


_serializer.serialize_start_tag = _serialize_start_tag
_serializer_api.serialize_start_tag = _serialize_start_tag

_original_find_open_index = ParseEngine._find_open_index
_original_find_open_html_index = ParseEngine._find_open_html_index


def _find_open_index(self, name):
    stack = self._stack
    if isinstance(stack, _CountingStack) and stack.count_of(name) == 0: return None
    return _original_find_open_index(self, name)


def _find_open_html_index(self, name):
    stack = self._stack
    if isinstance(stack, _CountingStack) and stack.count_of(name) == 0: return None
    return _original_find_open_html_index(self, name)


ParseEngine._find_open_index = _find_open_index
ParseEngine._find_open_html_index = _find_open_html_index


def _insert_fragment(self, fragment, reference_node):
    self._adopt_child(fragment)
    index = len(self.children) if reference_node is None else self.children.index(reference_node)
    children = fragment.children.copy()
    fragment.children.clear()
    for child in children: child.parent = self
    self.children[index:index] = children


def _append_child(self, node):
    if self.children is None: return
    if isinstance(node, DocumentFragment):
        self._insert_fragment(node, None)
        return
    self._adopt_child(node)
    self.children.append(node)
    node.parent = self


def _insert_before(self, node, reference_node):
    if self.children is None: raise ValueError(f"Node {self.name} cannot have children")
    if reference_node is None:
        self.append_child(node)
        return
    if node is reference_node: return
    try: index = self.children.index(reference_node)
    except ValueError: raise ValueError("Reference node is not a child of this node") from None
    if isinstance(node, DocumentFragment):
        self._insert_fragment(node, reference_node)
        return
    old_parent,old_index = self._adopt_child(node)
    if old_parent is self and old_index is not None and old_index < index: index -= 1
    self.children.insert(index, node)
    node.parent = self


def _replace_child(self, new_node, old_node):
    if self.children is None: raise ValueError(f"Node {self.name} cannot have children")
    try: index = self.children.index(old_node)
    except ValueError: raise ValueError("The node to be replaced is not a child of this node") from None
    if new_node is old_node: return old_node
    if isinstance(new_node, DocumentFragment):
        self._insert_fragment(new_node, old_node)
        self.remove_child(old_node)
        return old_node
    old_parent,old_index = self._adopt_child(new_node)
    if old_parent is self and old_index is not None and old_index < index: index -= 1
    self.children[index] = new_node
    new_node.parent = self
    old_node.parent = None
    return old_node


Node._insert_fragment = _insert_fragment
Node.append_child = _append_child
Node.insert_before = _insert_before
Node.replace_child = _replace_child


def parse_mdhtml(source):
    "Parse an MDHTML fragment into a mutable JustHTML DOM."
    return JustHTML(source, fragment_context=FragmentContext("body"), sanitize=False).root
