# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""The check every SVG passes before it leaves the renderer.

The portal shows the SVG in a sandboxed frame that runs no script and loads
nothing, and sanitizes it again; this is the renderer's own line. The SVG is
parsed with defusedxml (no DTD, no entities) and refused if anything in it
could run, navigate or load: an element outside a drawing allowlist, an `on*`
attribute, a reference that is not a `#fragment`, a `url(...)` that is not
`url(#fragment)`, or a style sheet importing another.

One rewrite is made first: the engine names its fonts with `@font-face`
rules pointing at `/static/fonts/…`, a server the frame does not have. They
are dropped; the portal supplies the same families from its own bundle.
"""

from __future__ import annotations

import re

from defusedxml import ElementTree
from defusedxml.common import DefusedXmlException

SVG_NS = "http://www.w3.org/2000/svg"
XLINK_NS = "http://www.w3.org/1999/xlink"

# What a chart is drawn with. Anything else (script, foreignObject, a, image,
# animate, set, iframe, …) refuses the whole SVG.
ALLOWED_ELEMENTS = frozenset(
    {
        "svg", "g", "defs", "title", "desc", "style",
        "path", "rect", "circle", "ellipse", "line", "polyline", "polygon",
        "text", "tspan",
        "clipPath", "mask", "pattern", "marker", "symbol", "use",
        "linearGradient", "radialGradient", "stop",
    }
)

FONT_FACE = re.compile(r"@font-face\s*\{[^{}]*\}")
URL_CALL = re.compile(r"url\s*\(\s*(['\"]?)\s*([^)'\"]*)", re.IGNORECASE)
ACTIVE_WORDS = ("javascript:", "vbscript:", "expression(", "@import", "behavior:", "-moz-binding")


class UnsafeSvg(ValueError):
    """The SVG holds something that could run, navigate or load."""


def strip_font_faces(svg: str) -> str:
    return FONT_FACE.sub("", svg)


def clean_svg(svg: str) -> str:
    """`svg` without the engine's font faces, once it passes the check."""
    cleaned = strip_font_faces(svg)
    try:
        root = ElementTree.fromstring(
            cleaned, forbid_dtd=True, forbid_entities=True, forbid_external=True
        )
    except (DefusedXmlException, ElementTree.ParseError, ValueError) as error:
        raise UnsafeSvg(f"not a well-formed SVG document: {error}") from None
    if root.tag != f"{{{SVG_NS}}}svg":
        raise UnsafeSvg(f"the document is <{root.tag}>, not an SVG")
    for element in root.iter():
        _check_element(element)
    return cleaned


def _local(name: str) -> tuple[str | None, str]:
    if name.startswith("{"):
        namespace, local = name[1:].split("}", 1)
        return namespace, local
    return None, name


def _check_element(element) -> None:
    namespace, tag = _local(element.tag)
    if namespace != SVG_NS or tag not in ALLOWED_ELEMENTS:
        raise UnsafeSvg(f"<{tag}> is not drawn by charts")
    for name, value in element.attrib.items():
        attribute_ns, attribute = _local(name)
        lowered = attribute.lower()
        if lowered.startswith("on"):
            raise UnsafeSvg(f"<{tag}> has an event handler, {attribute}")
        if lowered == "href" and attribute_ns in (None, XLINK_NS):
            if not value.startswith("#"):
                raise UnsafeSvg(f"<{tag}> links outside the document")
        elif attribute_ns not in (None, XLINK_NS):
            raise UnsafeSvg(f"<{tag}> has a foreign attribute, {attribute}")
        if not _is_text_attribute(lowered):
            _check_css(value, f"<{tag} {attribute}>")
    if tag == "style":
        _check_css(element.text or "", "<style>")
    # Text content is shown, not interpreted; the parser has already decoded it.


def _is_text_attribute(name: str) -> bool:
    """Attributes whose value is only ever read as text: labels the engine
    fills from the data, which may say anything and are never loaded."""
    return name.startswith(("aria-", "data-")) or name in ("id", "class", "role")


def _check_css(text: str, where: str) -> None:
    lowered = text.lower()
    for word in ACTIVE_WORDS:
        if word in lowered:
            raise UnsafeSvg(f"{where} holds {word}")
    for match in URL_CALL.finditer(text):
        if not match.group(2).startswith("#"):
            raise UnsafeSvg(f"{where} loads a URL")
