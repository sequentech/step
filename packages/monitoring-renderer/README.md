<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# monitoring-renderer

An internal service that draws monitoring widgets to SVG with
[dbt Charts](https://pypi.org/project/dbt-charts/) 0.8.0. Only Harvest calls
it. It is not reachable from a browser, it holds no data and it makes no
outbound connections. Harvest builds each board from the election's figures
and sends it here. The renderer checks the board against the same policy as
`sequent-core` (`src/monitoring/policy.rs`), draws it, checks the SVG and
returns it.

## API

Every route except `GET /health` and `GET /ready` needs the
`X-Renderer-Token` header. The token is compared in constant time. Anything
else, unknown routes included, gets a `401`. Every response is JSON with
`Cache-Control: no-store` and `X-Content-Type-Options: nosniff`.

### `GET /health`

Returns `200 {"status": "ok"}` as long as the process is up.

### `GET /ready`

Returns `200 {"status": "ready", "dbt_charts_version": "0.8.0"}` once the
engine has drawn its warm-up board. Before that it returns `503` with
`status` set to `starting`, or to `failed` if the warm-up failed. The first
draw takes about 2 s; later ones take 20–200 ms.

### `POST /render`

```json
{
  "board": { "charts": {}, "rows": [], "style": {}, "theme": "clarity", "queries": {} },
  "width": 640,
  "height": null,
  "color_scheme": "light",
  "locale": "en-US"
}
```

| Field | Rule |
| --- | --- |
| `board` | A dbt Charts board as a JSON object: `charts`, `rows`, `cols`, `grid`, `style`, `theme` and `queries` only. Queries are inline `{columns, values}`. |
| `width` | A JSON integer from 200 to 2400. The SVG is exactly this wide. |
| `height` | An integer from 80 to 4000, or `null`. It is a minimum height (`style.frame.min_height`); a board may draw taller. |
| `color_scheme` | `light` or `dark`. `dark` draws with dbt Charts' only dark theme, `neon` (page background `#161616`), whatever `theme` says. |
| `locale` | A BCP 47 tag, default `en`. dbt Charts 0.8.0 has no locale support, so this only keys the cache. Harvest formats numbers and labels before sending them. |

Unknown fields, strings for numbers and duplicate JSON keys are refused.

Success returns `200`:

```json
{ "svg": "<svg ...>", "render_ms": 85, "dbt_charts_version": "0.8.0", "cache": "MISS", "warnings": [] }
```

`cache` is `HIT` or `MISS`. The cache is an in-memory LRU keyed by the
SHA-256 of the canonical request plus the engine version. `warnings` are
problems with `severity: warning`. The board was still drawn. An example is
`WARN-CATEGORY-COLOR-PIN-UNSEEN`, which fires when a pinned category colour
is missing from this draw's data.

The renderer always turns the engine's footer and timestamp off.

### `POST /validate`

Takes `{"board": {...}}` and nothing else. It returns
`200 {"problems": [...]}`. The list holds the policy's problems first. If
those allow the board, it adds the engine's compile diagnostics and what
drawing the board reports. An empty list means the board draws cleanly.

### Problems

Problems have the shape of sequent-core's `monitoring::problem::Problem`:

```json
{ "severity": "error", "code": "forbidden_key", "path": "charts.k.link", "message": "...", "engine_code": null }
```

- `code` is one of `unreadable`, `too_large`, `forbidden_key`,
  `forbidden_value`, `invalid_value`, `duplicate_id`, `dangling_reference`
  or `chart_schema`.
- The engine's own diagnostics are `chart_schema`, and `engine_code` carries
  the dbt Charts code (for example `ERR-EXTRA-FIELD`).
- `RENDERER-UNSAFE-OUTPUT` means the drawn SVG failed the renderer's check.
- `RENDERER-ENGINE-FAILED` means the engine failed without a diagnostic.

### Status codes

| Status | Body | When |
| --- | --- | --- |
| 200 | as above | drawn / checked |
| 401 | `{"error": "UNAUTHORIZED"}` | missing or wrong token |
| 413 | `{"error": "TOO_LARGE"}` | body over 256 KiB (by `Content-Length` or as it streams) |
| 422 | `{"problems": [...]}` | malformed request, or a board the policy or the engine refuses |
| 504 | `{"error": "RENDER_TIMEOUT"}` | no slot, or no drawing, within the time limit |
| 500 | `{"error": "RENDER_FAILED" \| "VALIDATE_FAILED"}` | an unexpected engine failure (logged) |

Every error body also has a `message`.

## Configuration

| Variable | Default | |
| --- | --- | --- |
| `RENDERER_TOKEN` | none | Required. The service will not start if it is empty. Harvest sends the same value as `HARVEST_MONITORING_RENDERER_TOKEN`. |
| `RENDERER_CONCURRENCY` | `2` | Number of draws running at once. Other requests wait for a slot, within the time limit. |
| `RENDERER_TIMEOUT_MS` | `4000` | Time limit for waiting plus drawing, after which the request gets a `504`. A timed-out draw keeps its slot until it finishes. |
| `RENDERER_CACHE_BYTES` | `67108864` | Size of the SVG cache. |
| `RENDERER_HOST`, `RENDERER_PORT` | `0.0.0.0`, `8080` | Listen address. |

The compose files give Harvest `HARVEST_MONITORING_RENDERER_URL` (default
`http://monitoring-renderer:8080`), `HARVEST_MONITORING_RENDERER_TOKEN` and
`HARVEST_MONITORING_RENDERER_TIMEOUT_MS` (default `5000`, set above the
renderer's own limit).

## Security

- **Policy.** The board policy (`policy.py`) is a port of `policy.rs`, using
  the same key list (`data/dbt_charts_keys.txt`, a verbatim copy that a test
  keeps in step). It refuses:
  - SQL, files, URLs and links;
  - any theme that is not built in;
  - a visible footer or timestamp;
  - more than 50 charts, 16 queries, 5000 rows, 64 columns, 1024-character
    cells or 32 levels of nesting.
- **Engine.** One resident `ProjectSession` runs on an empty read-only
  project. DuckDB has external access and extension loading off.
- **Network.**
  - vl-convert runs with `allowed_base_urls=[]`, and its PNG, PDF, HTML and
    URL outputs are disabled.
  - An audit hook refuses every outbound socket.
  - In compose, the container sits alone with Harvest on an `internal`
    network.
- **Output.** The SVG is parsed with defusedxml, which forbids DTDs,
  entities and external references. Elements come from an allowlist.
  Scripts, event handlers, non-fragment `href`s, external `url()` and CSS
  imports are refused.
- **Container.** It runs as uid 10001 on a read-only root filesystem with a
  `/tmp` tmpfs. It drops all capabilities, sets `no-new-privileges` and
  publishes no ports.
- **Dependencies.** Every one is pinned with hashes (`requirements.txt`) and
  audited in CI with `pip-audit`.

## Fonts and maps

- **Fonts.**
  - The engine measures text with the fonts inside the dbt-charts wheel.
  - The SVG names the families `Inter Variable`, `Inter`, `dbt Sans Tabular`,
    `Source Serif 4` and `Noto Emoji`, followed by system fallbacks.
  - The engine's `@font-face` rules point at `/static/fonts/...`. The
    renderer strips them, so the page showing the SVG supplies the fonts or
    falls back to `system-ui` and `serif`.
- **Maps.**
  - The engine fetches `world-countries` and `world-50m` from
    vega-datasets. The upstream `world-50m.json` URL returns 404.
  - The renderer ships both as TopoJSON under `data/geo`, built from
    world-atlas 1.1.4 (BSD-3-Clause, Natural Earth data) by
    `scripts/build_geo.py`, and inlines them before drawing.
  - Any other map URL is refused.
- **Timestamps.** Each SVG carries a `data-rendered-at` attribute. A
  response served from the cache keeps the time of the first draw.

## Development

```sh
cd packages/monitoring-renderer
python3.12 -m venv .venv
.venv/bin/pip install --no-deps --require-hashes -r requirements.txt -r requirements-dev.txt
.venv/bin/python -m pytest -q          # sockets are disabled in every test
docker build -t monitoring-renderer .
docker run --rm --read-only --tmpfs /tmp --cap-drop ALL --security-opt no-new-privileges:true \
  -e RENDERER_TOKEN=dev -p 8080:8080 monitoring-renderer
```

The tests read the golden boards and the key list from
`packages/sequent-core/src/monitoring`.

To change a dependency:

1. Edit `requirements.in` or `requirements-dev.in`.
2. Run `pip-compile --generate-hashes --allow-unsafe --strip-extras` on each
   file.

The image is about 1.1 GB. The largest parts are pyarrow, vl-convert and
DuckDB, all dependencies of dbt Charts.
