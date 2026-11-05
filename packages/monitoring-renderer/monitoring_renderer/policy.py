# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""What the renderer refuses in a board, before the engine sees it.

Harvest only sends boards that sequent-core's policy accepted and that
`build_board` assembled, so nothing here should ever fire. It is the same
walk again, as defence in depth: a bug or a compromise upstream still cannot
make the engine read a file, fetch a URL, run SQL or draw markup.

It follows `sequent-core/src/monitoring/policy.rs`, from the same key list
(`data/dbt_charts_keys.txt`, a verbatim copy that a test compares), with what
only a board has: the governed rows as inline `values` queries, the engine
footer and freshness line switched off, and the theme by name. Paths and
codes are the Rust policy's, so both report a problem at the same place; the
shared fixture `fixtures/refused.yaml` tests that.
"""

from __future__ import annotations

import math
import re
import unicodedata
from dataclasses import dataclass
from importlib.resources import files
from typing import Any

from .problems import Code, Problem

KEYS_FILE = files("monitoring_renderer").joinpath("data/dbt_charts_keys.txt")

MAX_DEPTH = 32
MAX_CHARTS = 50
MAX_QUERIES = 16
MAX_ROWS = 5000
MAX_COLUMNS = 64
MAX_CELL_CHARS = 1024
MAX_DBT_COUNT = 100.0
MAX_DBT_NUMBER = 10_000.0

# The built-in themes dbt-charts 0.8.0 ships (not its diagnostics themes).
THEMES = ("clarity", "neon", "paper", "stark", "vivid")

# What a board may hold: what a widget's chart may set, plus what
# `build_board` adds.
CHART_FIELDS = ("charts", "rows", "cols", "grid", "style")
BOARD_FIELDS = CHART_FIELDS + ("theme", "queries")
NESTED_BOARD_FIELDS = ("rows", "cols", "grid", "style", "title", "visible", "card_gap", "height", "width")
LAYOUT_FIELDS = ("rows", "cols", "grid")
NAVIGATING_FIELDS = ("tabs", "details")
GRID_FIELDS = ("columns", "items")
GRID_ITEM_FIELDS = ("item", "col", "col_span", "row", "row_span", "height", "width")
BOARD_STYLE_FIELDS = (
    "accent", "background", "border", "box_shadow", "charts", "font", "footer", "formats",
    "frame", "gap", "layout", "margin", "muted", "opacity", "padding", "palettes",
    "placeholder", "roles", "text", "timestamp", "title", "tones", "variables",
)
# `build_board` switches these off; they may be there only switched off.
HIDDEN_STYLE_FIELDS = ("footer", "timestamp")
COLOUR_KEYS = (
    "accent", "background", "color", "color_active", "color_disabled", "color_inactive",
    "drop_line_color", "fill", "focus_color", "glyph_color", "info", "muted", "negative",
    "null_color", "palette", "positive", "static", "stroke", "warning",
)
AFFIX_KEYS = ("glyph", "prefix", "suffix", "value_suffix")
COUNT_KEYS = (
    "bin_maxbins", "col", "col_span", "columns", "compact_columns", "count", "label_max_lines",
    "level", "max_bars", "max_chars", "max_number", "page_rows", "row", "row_span", "symbol_limit",
)
DATA_VALUED_KEYS = (
    "bound", "domain", "gt", "gte", "lt", "lte", "max", "min", "range", "thresholds", "values", "x", "y",
)
FORBIDDEN_CHART_TYPES = ("callout", "image")
GEO_SOURCES = ("world-countries", "world-50m")
PLAIN_TEXT_PARENTS = ("footer", "overlay")
COLUMN_SOURCE_PARENTS = ("support_table", "entries")
TEMPLATE_MARKERS = ("{{", "{%", "{#", "${")
SCRIPT_SCHEMES = ("javascript:", "vbscript:")
URL_SCHEMES = (
    "about", "blob", "data", "file", "ftp", "http", "https", "intent", "mailto", "sms", "tel", "ws", "wss",
)
NAME = re.compile(r"[A-Za-z0-9_]{1,64}")
QUERY_NAME = re.compile(r"[A-Za-z0-9_-]{1,64}")
RULES = {"allow", "forbid", "chart_type", "flag", "geo", "layout", "names", "query", "source", "text"}


def _load_rules() -> dict[str, str]:
    rules = {}
    for line in KEYS_FILE.read_text(encoding="utf-8").splitlines():
        if not line.strip() or line.startswith("#"):
            continue
        words = line.split()
        rule = words[1] if len(words) > 1 else ""
        # A misspelt rule forbids the key, as in Rust.
        rules[words[0]] = rule if rule in RULES else "forbid"
    return rules


KEY_RULES = _load_rules()


@dataclass
class Governed:
    queries: list[str]
    charts: list[str]


class _Report:
    def __init__(self) -> None:
        self.problems: list[Problem] = []

    def error(self, code: Code, path: str, message: str) -> None:
        self.problems.append(Problem.error(code, path, message))


def check_board(board: Any) -> list[Problem]:
    """Every problem with `board`; empty when it may be drawn."""
    report = _Report()
    if not isinstance(board, dict):
        report.error(Code.INVALID_VALUE, "", "A board is a mapping.")
        return report.problems
    if _depth(board) > MAX_DEPTH + 4:  # queries add rows of cells
        report.error(Code.TOO_LARGE, "", f"A board nests at most {MAX_DEPTH} levels deep.")
        return report.problems
    configuration = {key: value for key, value in board.items() if key != "queries"}
    if _depth(configuration) > MAX_DEPTH:
        report.error(Code.TOO_LARGE, "", f"A board nests at most {MAX_DEPTH} levels deep.")
        return report.problems
    _scan_text(configuration, "", report)
    queries = _check_queries(board.get("queries"), report)
    charts = board.get("charts")
    governed = Governed(
        queries=queries,
        charts=list(charts) if isinstance(charts, dict) else [],
    )
    if isinstance(charts, dict) and len(charts) > MAX_CHARTS:
        report.error(Code.TOO_LARGE, "charts", f"A board draws at most {MAX_CHARTS} charts.")
    for key, value in board.items():
        if key == "charts":
            _walk_charts(value, "charts", governed, report)
        elif key in ("rows", "cols", "grid", "style"):
            _walk_board_field(key, value, key, governed, report, top=True)
        elif key == "theme":
            if value not in THEMES:
                report.error(Code.INVALID_VALUE, "theme", f"A board's theme is one of {', '.join(THEMES)}.")
        elif key == "queries":
            pass
        elif key in NAVIGATING_FIELDS:
            report.error(Code.FORBIDDEN_KEY, key, _navigating(key))
        else:
            report.error(
                Code.FORBIDDEN_KEY,
                key,
                f"A board may only set {', '.join(BOARD_FIELDS)}; '{key}' is supplied by the renderer or not allowed.",
            )
    return report.problems


# -- the governed rows ------------------------------------------------------


def _check_queries(queries: Any, report: _Report) -> list[str]:
    if not isinstance(queries, dict) or not queries:
        report.error(Code.INVALID_VALUE, "queries", "A board carries its widget's governed rows as queries.")
        return []
    if len(queries) > MAX_QUERIES:
        report.error(Code.TOO_LARGE, "queries", f"A board carries at most {MAX_QUERIES} queries.")
    names = []
    for name, query in queries.items():
        path = _join("queries", name)
        names.append(name)
        if not QUERY_NAME.fullmatch(name):
            report.error(Code.INVALID_VALUE, path, "A query name is letters, digits, '_' and '-'.")
        if not isinstance(query, dict):
            report.error(Code.FORBIDDEN_VALUE, path, "A query is inline rows: `columns` and `values`.")
            continue
        for key in query:
            if key not in ("columns", "values"):
                report.error(
                    Code.FORBIDDEN_KEY,
                    _join(path, key),
                    "A query is inline rows only; it cannot run SQL, read a file or fetch a URL.",
                )
        columns = query.get("columns")
        values = query.get("values")
        if not isinstance(columns, list) or not isinstance(values, list):
            report.error(Code.INVALID_VALUE, path, "A query has a list of `columns` and a list of `values` rows.")
            continue
        if len(columns) > MAX_COLUMNS:
            report.error(Code.TOO_LARGE, _join(path, "columns"), f"A query has at most {MAX_COLUMNS} columns.")
        for position, column in enumerate(columns):
            if not isinstance(column, str) or not NAME.fullmatch(column):
                report.error(
                    Code.INVALID_VALUE,
                    _index(_join(path, "columns"), position),
                    "A column name is letters, digits and '_'.",
                )
        if len(values) > MAX_ROWS:
            report.error(Code.TOO_LARGE, _join(path, "values"), f"A query has at most {MAX_ROWS} rows.")
            continue
        for position, row in enumerate(values):
            row_path = _index(_join(path, "values"), position)
            if not isinstance(row, list) or len(row) != len(columns):
                report.error(Code.INVALID_VALUE, row_path, "A row has one value for each column.")
                continue
            for cell_position, cell in enumerate(row):
                if not _is_cell(cell):
                    report.error(
                        Code.INVALID_VALUE,
                        _index(row_path, cell_position),
                        f"A value is a number, text of at most {MAX_CELL_CHARS} characters, true, false or null.",
                    )
    return names


def _is_cell(cell: Any) -> bool:
    if cell is None or isinstance(cell, bool):
        return True
    if isinstance(cell, (int, float)):
        return not isinstance(cell, float) or math.isfinite(cell)
    return isinstance(cell, str) and len(cell) <= MAX_CELL_CHARS


# -- paths ------------------------------------------------------------------


def _join(path: str, key: str) -> str:
    return key if not path else f"{path}.{key}"


def _index(path: str, position: int) -> str:
    return f"{path}[{position}]"


def _depth(value: Any) -> int:
    # Iterative, so a hostile board cannot exhaust the stack.
    deepest = 0
    stack = [(value, 0)]
    while stack:
        node, level = stack.pop()
        if isinstance(node, dict):
            children = node.values()
        elif isinstance(node, list):
            children = node
        else:
            deepest = max(deepest, level)
            continue
        deepest = max(deepest, level + 1)
        if level + 1 > MAX_DEPTH + 8:
            return level + 1
        stack.extend((child, level + 1) for child in children)
    return deepest


# -- the content pass: every string, key or value ---------------------------


def _scan_text(value: Any, path: str, report: _Report) -> None:
    if isinstance(value, str):
        what = interpreted(value)
        if what:
            report.error(
                Code.FORBIDDEN_VALUE,
                path,
                f"Text may not contain {what}; it would be interpreted rather than shown.",
            )
    elif isinstance(value, list):
        for position, item in enumerate(value):
            _scan_text(item, _index(path, position), report)
    elif isinstance(value, dict):
        for key, item in value.items():
            child = _join(path, key)
            _scan_text(key, child, report)
            _scan_text(item, child, report)


def _is_control(char: str) -> bool:
    return unicodedata.category(char) == "Cc"


def interpreted(text: str) -> str | None:
    """What in `text` would be evaluated or followed downstream, if anything."""
    if any(marker in text for marker in TEMPLATE_MARKERS):
        return "a template"
    for first, second in zip(text, text[1:]):
        if first == "<" and ((second.isascii() and second.isalpha()) or second in "/!?"):
            return "markup"
    if "](" in text:
        return "a link"
    lower = text.lower()
    without_controls = "".join(c for c in lower if not _is_control(c))
    squeezed = "".join(c for c in without_controls if not c.isspace())
    if any(scheme in squeezed for scheme in SCRIPT_SCHEMES):
        return "a script URL"
    for match in re.finditer(re.escape("url("), squeezed):
        at = match.start()
        if at == 0 or not squeezed[at - 1].isalnum():
            return "a CSS URL"
    controls_as_spaces = "".join(" " if _is_control(c) else c for c in lower)
    separators = re.compile(r"[\s\"'()`\[\],;]")
    for candidate in (without_controls, controls_as_spaces):
        if any(_address(word) for word in separators.split(candidate) if word):
            return "a URL"
    if _has_url(without_controls):
        return "a URL"
    return None


def _address(word: str) -> bool:
    if ":" in word:
        scheme, rest = word.split(":", 1)
        if scheme == "data":
            addressed = bool(rest) and ((rest[0].isascii() and rest[0].isalpha()) or rest[0] == ",")
        else:
            addressed = bool(rest)
        if scheme in URL_SCHEMES and addressed:
            return True
    for prefix in ("//", "www."):
        if word.startswith(prefix):
            rest = word[len(prefix):]
            if rest and rest[0].isalnum():
                return True
    return False


def _has_url(text: str) -> bool:
    start = 0
    while True:
        at = text.find("://", start)
        if at < 0:
            return False
        scheme = []
        for char in reversed(text[:at]):
            if char.isascii() and (char.isalnum() or char in "+.-"):
                scheme.append(char)
            else:
                break
        if scheme and scheme[-1].isascii() and scheme[-1].isalpha():
            return True
        start = at + 1


# -- the board's structure ------------------------------------------------


def _navigating(key: str) -> str:
    return (
        f"'{key}' switches views by following a link, which a widget's frame cannot do; "
        "give the widget a selector instead."
    )


def _walk_charts(value: Any, path: str, governed: Governed, report: _Report) -> None:
    if not isinstance(value, dict):
        report.error(Code.INVALID_VALUE, path, "`charts` maps chart names to chart definitions.")
        return
    for name, chart in value.items():
        child = _join(path, name)
        if isinstance(chart, dict):
            _walk_dbt(chart, child, name, governed, report)
        else:
            report.error(
                Code.FORBIDDEN_VALUE,
                child,
                "A chart is defined here in full; it cannot be taken from another file.",
            )


def _walk_board_field(key: str, value: Any, path: str, governed: Governed, report: _Report, top: bool = False) -> None:
    if key in ("rows", "cols"):
        _walk_layout(value, path, governed, report)
    elif key == "grid":
        _walk_grid(value, path, governed, report)
    elif key == "style":
        _walk_board_style(value, path, governed, report, top=top)
    elif key == "visible":
        _check_flag(key, value, "", path, report)
    else:
        _walk_dbt(value, path, key, governed, report)


def _walk_board_style(value: Any, path: str, governed: Governed, report: _Report, top: bool) -> None:
    if not isinstance(value, dict):
        _walk_dbt(value, path, "style", governed, report)
        return
    rest = {}
    for key, item in value.items():
        child = _join(path, key)
        if key not in BOARD_STYLE_FIELDS:
            report.error(
                Code.FORBIDDEN_KEY,
                child,
                f"A board's style may set {', '.join(BOARD_STYLE_FIELDS)}; '{key}' is not allowed.",
            )
        elif top and key in HIDDEN_STYLE_FIELDS:
            if item != {"visible": False}:
                report.error(Code.FORBIDDEN_KEY, child, _forbidden_because(key))
            continue
        rest[key] = item
    _walk_dbt(rest, path, "style", governed, report)


def _walk_layout(value: Any, path: str, governed: Governed, report: _Report) -> None:
    if value is None:
        return
    if not isinstance(value, list):
        report.error(Code.INVALID_VALUE, path, "A layout lists the names of the widget's charts.")
        return
    for position, item in enumerate(value):
        child = _index(path, position)
        if isinstance(item, str):
            _place(item, child, governed, report)
        elif isinstance(item, dict):
            _walk_nested_board(item, child, governed, report)
        else:
            report.error(Code.INVALID_VALUE, child, "A layout lists the names of the widget's charts.")


def _place(name: str, path: str, governed: Governed, report: _Report) -> None:
    if name not in governed.charts:
        report.error(
            Code.DANGLING_REFERENCE,
            path,
            f"'{name}' is not one of this board's charts ({', '.join(governed.charts)}).",
        )


def _walk_nested_board(value: Any, path: str, governed: Governed, report: _Report) -> None:
    if not isinstance(value, dict):
        report.error(Code.INVALID_VALUE, path, "A board inside the layout is a mapping of rows, cols or grid.")
        return
    if not any(key in LAYOUT_FIELDS for key in value):
        report.error(
            Code.INVALID_VALUE,
            path,
            "A board inside the layout places the widget's charts with rows, cols or grid.",
        )
    for key, item in value.items():
        child = _join(path, key)
        scalar = item is None or isinstance(item, str) or (isinstance(item, (int, float)) and not isinstance(item, bool))
        if key == "title" and not (item is None or isinstance(item, str)):
            report.error(Code.INVALID_VALUE, child, "A board's title is text.")
        elif key in ("card_gap", "height", "width") and not scalar:
            report.error(Code.INVALID_VALUE, child, f"'{key}' is a size.")
        elif key in NESTED_BOARD_FIELDS:
            _walk_board_field(key, item, child, governed, report)
        elif key in NAVIGATING_FIELDS:
            report.error(Code.FORBIDDEN_KEY, child, _navigating(key))
        elif key in ("charts", "type"):
            report.error(
                Code.FORBIDDEN_KEY,
                child,
                "Define charts once, under charts, and place them here by name.",
            )
        else:
            report.error(
                Code.FORBIDDEN_KEY,
                child,
                f"A board inside the layout may set {', '.join(NESTED_BOARD_FIELDS)}; '{key}' is not allowed.",
            )


def _walk_grid(value: Any, path: str, governed: Governed, report: _Report) -> None:
    if not isinstance(value, dict):
        report.error(Code.INVALID_VALUE, path, "A grid is a mapping of `columns` and `items`.")
        return
    for key, item in value.items():
        child = _join(path, key)
        if key == "items":
            if not isinstance(item, list):
                report.error(Code.INVALID_VALUE, child, "A grid's items are a list of cells.")
                continue
            for position, cell in enumerate(item):
                _walk_grid_item(cell, _index(child, position), governed, report)
        elif key in GRID_FIELDS:
            _walk_dbt(item, child, key, governed, report)
        else:
            report.error(Code.FORBIDDEN_KEY, child, f"A grid may set {', '.join(GRID_FIELDS)}; '{key}' is not allowed.")


def _walk_grid_item(value: Any, path: str, governed: Governed, report: _Report) -> None:
    if not isinstance(value, dict):
        report.error(Code.INVALID_VALUE, path, "A grid cell is a mapping with the `item` it holds and where.")
        return
    if "item" not in value:
        report.error(Code.INVALID_VALUE, path, "A grid cell names the `item` it holds.")
    for key, item in value.items():
        child = _join(path, key)
        if key == "item":
            if isinstance(item, str):
                _place(item, child, governed, report)
            elif isinstance(item, dict):
                _walk_nested_board(item, child, governed, report)
            else:
                report.error(Code.INVALID_VALUE, child, "A grid cell holds the name of one of the board's charts.")
        elif key in GRID_ITEM_FIELDS:
            _walk_dbt(item, child, key, governed, report)
        else:
            report.error(
                Code.FORBIDDEN_KEY,
                child,
                f"A grid cell may set {', '.join(GRID_ITEM_FIELDS)}; '{key}' is not allowed.",
            )


# -- chart definitions and styles -------------------------------------------


def _walk_dbt(value: Any, path: str, parent: str, governed: Governed, report: _Report) -> None:
    if isinstance(value, dict):
        for key, item in value.items():
            _walk_dbt_key(key, item, _join(path, key), parent, governed, report)
    elif isinstance(value, list):
        for position, item in enumerate(value):
            _walk_dbt(item, _index(path, position), parent, governed, report)
    elif isinstance(value, str):
        _check_dbt_text(value, path, parent, report)
    elif isinstance(value, (int, float)) and not isinstance(value, bool):
        _check_dbt_number(value, path, parent, report)


def _in_style(path: str) -> bool:
    return any(segment == "style" or segment.startswith("style[") for segment in path.split("."))


def _check_dbt_text(text: str, path: str, parent: str, report: _Report) -> None:
    if '"' in text or "\\" in text:
        report.error(
            Code.FORBIDDEN_VALUE,
            path,
            "Chart text may not contain a double quote or a backslash; use typographic quotes (“ ”).",
        )
        return
    style = _in_style(path)
    if style and any(char in text for char in ";{}"):
        report.error(Code.FORBIDDEN_VALUE, path, "A style value is one value; ';', '{' and '}' would start another.")
        return
    colour = parent in COLOUR_KEYS or parent == "thresholds" or (parent == "values" and ".category_colors." in path)
    if style and colour and not is_colour(text):
        report.error(
            Code.FORBIDDEN_VALUE,
            path,
            f"'{text}' is not a colour: use #rrggbb, rgb(), hsl(), a colour name or a theme token.",
        )
    if parent in AFFIX_KEYS and any(char.isascii() and char.isdigit() for char in text):
        report.error(
            Code.FORBIDDEN_VALUE,
            path,
            f"'{parent}' is written beside a figure, so it may not contain digits.",
        )


def is_colour(text: str) -> bool:
    if text.startswith("#"):
        hexa = text[1:]
        return len(hexa) in (3, 4, 6, 8) and all(c in "0123456789abcdefABCDEF" for c in hexa)
    for function in ("rgb(", "rgba(", "hsl(", "hsla("):
        if text.startswith(function) and text.endswith(")"):
            inner = text[len(function):-1]
            return all((c.isascii() and c.isdigit()) or c in ".,% /-" for c in inner)
    position = None
    name = text
    if text.endswith("]"):
        if "[" not in text[:-1]:
            return False
        name, position = text[:-1].split("[", 1)
    if position is not None and not (position and all(c.isascii() and c.isdigit() for c in position)):
        return False
    return (
        bool(name)
        and name[0].isascii()
        and name[0].isalpha()
        and all((c.isascii() and c.isalnum()) or c in "-_." for c in name)
    )


def _check_dbt_number(value: float, path: str, parent: str, report: _Report) -> None:
    magnitude = abs(value) if math.isfinite(value) else math.inf
    if parent in COUNT_KEYS and magnitude > MAX_DBT_COUNT:
        report.error(Code.TOO_LARGE, path, f"'{parent}' is at most {MAX_DBT_COUNT:g}.")
    elif parent not in DATA_VALUED_KEYS and not magnitude <= MAX_DBT_NUMBER:
        report.error(Code.TOO_LARGE, path, f"Sizes in a chart are at most {MAX_DBT_NUMBER:g}.")


def _walk_dbt_key(key: str, value: Any, path: str, parent: str, governed: Governed, report: _Report) -> None:
    rule = KEY_RULES.get(key)
    if rule is None:
        report.error(Code.FORBIDDEN_KEY, path, f"'{key}' is not a dbt Charts field the platform accepts.")
    elif rule == "allow":
        _walk_dbt(value, path, key, governed, report)
    elif rule == "forbid":
        report.error(Code.FORBIDDEN_KEY, path, _forbidden_because(key))
    elif rule == "layout":
        report.error(Code.FORBIDDEN_KEY, path, f"'{key}' belongs in the board's layout.")
    elif rule == "names":
        if isinstance(value, dict):
            for name, item in value.items():
                child = _join(path, name)
                if '"' in name or "\\" in name:
                    report.error(Code.FORBIDDEN_VALUE, child, "A name may not contain a double quote or a backslash.")
                _walk_dbt(item, child, key, governed, report)
        else:
            _walk_dbt(value, path, key, governed, report)
    elif rule == "query":
        _walk_query(value, path, governed, report)
    elif rule == "chart_type":
        if isinstance(value, str) and value in FORBIDDEN_CHART_TYPES:
            report.error(Code.FORBIDDEN_VALUE, path, f"A '{value}' chart shows typed-in content, not governed data.")
    elif rule == "flag":
        _check_flag(key, value, parent, path, report)
    elif rule == "geo" or (rule == "source" and parent == "basemap"):
        _check_geo(value, path, report)
    elif rule == "source" and parent in COLUMN_SOURCE_PARENTS:
        if not (isinstance(value, str) and NAME.fullmatch(value)):
            report.error(Code.FORBIDDEN_VALUE, path, "A support table reads a column of the board's query, by name.")
    elif rule == "source":
        report.error(
            Code.FORBIDDEN_KEY,
            path,
            "A chart reads its board's governed queries; it cannot name a source of its own.",
        )
    elif rule == "text":
        if isinstance(value, str) and parent in PLAIN_TEXT_PARENTS:
            _check_dbt_text(value, path, parent, report)
        elif isinstance(value, str):
            report.error(
                Code.FORBIDDEN_KEY,
                path,
                "Markdown text is not accepted; titles and labels say what a chart shows.",
            )
        else:
            _walk_dbt(value, path, key, governed, report)


def _forbidden_because(key: str) -> str:
    if key == "step":
        return "A tick step turns the data's range into any number of ticks; set `ticks.count` instead."
    if key == "aggregate":
        return "The renderer does not aggregate: every figure comes from the data source, counted once."
    if key in ("footer", "timestamp"):
        return "The dashboard says when its figures are from; the engine's footer and freshness line stay off."
    return f"'{key}' would bring data, a destination, markup or an expression from outside the governed sources."


def _check_flag(key: str, value: Any, parent: str, path: str, report: _Report) -> None:
    if not (value is None or isinstance(value, bool)):
        report.error(
            Code.FORBIDDEN_VALUE,
            path,
            f"'{key}' is true or false; an expression or a query probe would be evaluated.",
        )
    elif parent == "total" and value is True:
        report.error(
            Code.FORBIDDEN_VALUE,
            path,
            "The renderer does not add slices up: groups need not sum to the scope.",
        )


def _check_geo(value: Any, path: str, report: _Report) -> None:
    if not (value is None or (isinstance(value, str) and value in GEO_SOURCES)):
        report.error(
            Code.FORBIDDEN_VALUE,
            path,
            f"Maps draw the geometry bundled with the renderer: {', '.join(GEO_SOURCES)}.",
        )


def _walk_query(value: Any, path: str, governed: Governed, report: _Report) -> None:
    if isinstance(value, str):
        if value not in governed.queries:
            report.error(
                Code.DANGLING_REFERENCE,
                path,
                f"'{value}' is not one of this board's queries ({', '.join(governed.queries)}).",
            )
    elif isinstance(value, dict):
        report.error(
            Code.FORBIDDEN_KEY,
            path,
            "A chart reads one of its board's governed queries by name; it cannot define its own.",
        )
        _walk_dbt(value, path, "query", governed, report)
    else:
        report.error(Code.INVALID_VALUE, path, "A chart's query is the name of one of its board's queries.")
