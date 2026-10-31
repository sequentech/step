# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""The check every SVG passes before it leaves the renderer."""

from __future__ import annotations

import pytest

from monitoring_renderer.svg import UnsafeSvg, clean_svg

NS = 'xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"'


def svg(body: str, style: str = "") -> str:
    return f'<svg {NS} width="10" height="10"><style type="text/css">{style}</style>{body}</svg>'


def test_a_plain_chart_passes_and_keeps_its_fragment_references():
    text = svg(
        '<defs><clipPath id="c1"><rect width="5" height="5"/></clipPath>'
        '<linearGradient id="g"><stop offset="0" stop-color="#fff"/></linearGradient></defs>'
        '<g clip-path="url(#c1)"><path d="M0,0h5" fill="url(#g)" style="fill: url(#g)"/>'
        '<use href="#c1"/><text aria-label="x">5 &lt; 6</text></g>'
    )
    assert clean_svg(text) == text


def test_the_engines_font_faces_are_dropped_and_nothing_else():
    style = (
        "@font-face {\n  font-family: 'Inter Variable';\n"
        "  src: url('/static/fonts/InterVariable.woff2') format('woff2-variations');\n}\n"
        "svg text { font-variant-emoji: text; }"
    )
    cleaned = clean_svg(svg("<g/>", style))
    assert "@font-face" not in cleaned
    assert "/static/fonts" not in cleaned
    assert "font-variant-emoji" in cleaned


@pytest.mark.parametrize(
    "body",
    [
        "<script>alert(1)</script>",
        '<foreignObject><div xmlns="http://www.w3.org/1999/xhtml">x</div></foreignObject>',
        '<g onload="alert(1)"/>',
        '<rect ONCLICK="alert(1)"/>',
        '<a href="https://example.com"><text>x</text></a>',
        '<use href="https://example.com/s.svg#x"/>',
        '<use xlink:href="data:image/svg+xml,&lt;svg/&gt;"/>',
        '<image href="#x"/>',
        '<rect fill="url(https://example.com/x)"/>',
        '<rect style="fill: url( \'//example.com/x\' )"/>',
        '<rect style="background: url(data:image/png;base64,AAAA)"/>',
        '<animate attributeName="href" to="javascript:alert(1)"/>',
        '<set attributeName="onclick" to="alert(1)"/>',
        '<rect fill="javascript:alert(1)"/>',
        '<html:div xmlns:html="http://www.w3.org/1999/xhtml"/>',
    ],
)
def test_active_or_external_content_is_refused(body):
    with pytest.raises(UnsafeSvg):
        clean_svg(svg(body))


@pytest.mark.parametrize(
    "style",
    [
        "@import url('https://example.com/x.css');",
        "rect { fill: url(https://example.com/x); }",
        "rect { background: url(data:image/png;base64,AAAA); }",
        "rect { behavior: expression(alert(1)); }",
    ],
)
def test_style_sheets_reaching_elsewhere_are_refused(style):
    with pytest.raises(UnsafeSvg):
        clean_svg(svg("<g/>", style))


def test_entities_and_doctypes_are_refused():
    bomb = (
        '<?xml version="1.0"?><!DOCTYPE svg [<!ENTITY a "aaaa"><!ENTITY b "&a;&a;">]>'
        f"<svg {NS}><text>&b;</text></svg>"
    )
    with pytest.raises(UnsafeSvg):
        clean_svg(bomb)


def test_only_an_svg_document_passes():
    with pytest.raises(UnsafeSvg):
        clean_svg('<html xmlns="http://www.w3.org/1999/xhtml"/>')
    with pytest.raises(UnsafeSvg):
        clean_svg("not xml")
    with pytest.raises(UnsafeSvg):
        clean_svg("<svg><g/></svg>")  # no SVG namespace


def test_the_refusal_says_what_was_found():
    with pytest.raises(UnsafeSvg) as refused:
        clean_svg(svg("<script/>"))
    assert "script" in str(refused.value)
