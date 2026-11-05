# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""Maps without the internet.

dbt Charts hands vl-convert a Vega-Lite spec whose map data is a URL on
vega.github.io. The renderer wraps vl-convert so that:

- a URL of a world map the renderer bundles is replaced by that geometry,
  inline (`data/geo/`, Natural Earth via world-atlas 1.1.4);
- every conversion runs with `allowed_base_urls=[]`, so vl-convert refuses
  any other URL outright, whatever the spec says;
- conversions the renderer never asks for (PNG, PDF, JPEG, HTML, URLs) are
  switched off.

A spec with any other URL fails to draw, and the board is refused.
"""

from __future__ import annotations

import copy
import functools
import json
import threading
from importlib.resources import files
from typing import Any

GEO_DIR = files("monitoring_renderer").joinpath("data/geo")

# The URLs dbt-charts 0.8.0 fetches for the map geometry the policy allows
# (`world-countries`, `world-50m`), and the bundled file for each. A test
# reads the engine's geo defaults, so an upgrade that moves them fails there.
BUNDLED_URLS = {
    "https://vega.github.io/vega-datasets/data/world-110m.json": "world-110m.json",
    "https://vega.github.io/vega-datasets/data/world-50m.json": "world-50m.json",
}

# The vl-convert functions that draw (and would load data). Each is wrapped.
DRAWING = ("vegalite_to_svg", "vegalite_to_scenegraph", "vega_to_svg", "vega_to_scenegraph")
# The ones the renderer never uses: disabled.
DISABLED = (
    "vegalite_to_png", "vegalite_to_jpeg", "vegalite_to_pdf", "vegalite_to_html", "vegalite_to_url",
    "vega_to_png", "vega_to_jpeg", "vega_to_pdf", "vega_to_html", "vega_to_url",
    "svg_to_png", "svg_to_jpeg", "svg_to_pdf",
)

_installed = False
_lock = threading.Lock()


@functools.cache
def geometry(name: str) -> Any:
    return json.loads(GEO_DIR.joinpath(name).read_text(encoding="utf-8"))


class ExternalData(ValueError):
    """A spec asked for data from somewhere."""


def inline_bundled_data(spec: Any) -> Any:
    """`spec` with each bundled-map URL replaced by the map, inline."""
    if isinstance(spec, str):
        spec = json.loads(spec)
    else:
        spec = copy.deepcopy(spec)

    def walk(node: Any) -> None:
        if isinstance(node, dict):
            url = node.get("url")
            if isinstance(url, str):
                name = BUNDLED_URLS.get(url)
                if name is None:
                    raise ExternalData(f"the renderer does not load data from {url!r}")
                del node["url"]
                node["values"] = geometry(name)
                return
            for value in node.values():
                walk(value)
        elif isinstance(node, list):
            for value in node:
                walk(value)

    walk(spec)
    return spec


def _offline(function):
    @functools.wraps(function)
    def draw(spec, *args, **kwargs):
        kwargs["allowed_base_urls"] = []
        return function(inline_bundled_data(spec), *args, **kwargs)

    draw.__renderer_offline__ = True
    return draw


def _disabled(name: str):
    def refuse(*_args, **_kwargs):
        raise RuntimeError(f"vl_convert.{name} is disabled in the renderer")

    refuse.__renderer_offline__ = True
    return refuse


def install_offline_geo() -> None:
    """Wrap vl-convert for the whole process; idempotent."""
    global _installed
    import vl_convert

    with _lock:
        if _installed:
            return
        for name in DRAWING:
            function = getattr(vl_convert, name, None)
            if function is not None and not getattr(function, "__renderer_offline__", False):
                setattr(vl_convert, name, _offline(function))
        for name in DISABLED:
            if hasattr(vl_convert, name):
                setattr(vl_convert, name, _disabled(name))
        _installed = True
