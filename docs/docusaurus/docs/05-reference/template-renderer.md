<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->
# Template rendering and ZIP bundles

`packages/template-renderer` owns the pure Handlebars renderer used by native reports and Template Studio's browser/server WASM bindings. `sequent-core::services::reports` re-exports its helpers, asset validation and ZIP codec. Windmill and Velvet retain the production helper and wrapper composition. HTML generation does not require Chromium; executing attached JavaScript and printing PDFs does.

## Attached files

The existing template JSON can include an `assets` map. No database migration is needed:

```json
{
  "document": "<img src=\"/images/logo.svg\"><script src=\"/scripts/report.js\"></script>",
  "assets": {
    "images/logo.svg": { "mime": "image/svg+xml", "base64": "…" },
    "scripts/report.js": { "mime": "text/javascript", "base64": "…" }
  }
}
```

Use root-relative paths such as `/images/logo.svg` in HTML. CSS imports, CSS image/font URLs, ES-module imports and `fetch()` resolve relative to their file or the document root. Files are served from an in-memory map at a synthetic HTTPS origin. MIME types must be plain `type/subtype` values. Extensions do not grant execution privileges.

For asynchronous document construction, assign a promise during script execution:

```js
window.sequentTemplateReady = (async () => {
  const values = await fetch('/data/results.json').then(response => response.json());
  document.querySelector('#total').textContent = values.total;
})();
```

Rendering waits for the document load, this optional promise, fonts and images. Script errors or a rejected readiness promise fail the render. Animation timers and unregistered background work are not a readiness contract.

The shared renderer carries validated files in an inert `sequent-template-assets:v1` HTML comment. This preserves the existing HTML-only PDF interfaces, including Windmill, Velvet and the Lambda/OpenWhisk transports. The PDF receiver removes the envelope and serves files locally. Asset-bearing documents run in a fresh offline Chromium process with no application session, a sandbox response policy and interception that fulfils only the initial document and attached file paths. External/file URLs, frames, workers, forms, popups and subsequent navigation are blocked. Scripts are disabled after readiness. A process deadline stops looping scripts. Existing documents without a file envelope retain the legacy platform PDF path.

Studio uses the same file contract. It executes attachments outside the application browser, then serializes a static snapshot: scripts are removed, CSS imports and file URLs are inlined, and canvases become images. Its application iframe keeps scripts disabled. References uploaded for design inspiration are separate and are never executed as template attachments.

## ZIP exchange, version 1

Admin Portal template import/export and Studio use `.zip`, with `application/zip`. A bundle contains `manifest.json` and the files it declares:

```text
manifest.json
templates/0001/template.hbs
templates/0001/files/images/logo.svg
templates/0001/files/scripts/report.js
```

The manifest has `format: "sequent-template-bundle"`, `version: 1`, and a `templates` array. Each entry contains:

- `record`: `alias`, `tenant_id`, `created_by`, `communication_method`, `type`, optional labels/annotations/timestamps, and `template` settings. Source and attachments are outside these settings.
- `document`: the ZIP path of its UTF-8 Handlebars source, omitted for a template without a document.
- `files`: a map from a template-relative path to `{ "file": "ZIP entry path", "mime": "type/subtype" }`.

Files contain their real bytes in the ZIP; base64 is used only in JSON storage/API transport. Labels and annotations preserve JSON types. Localized exports keep runtime Handlebars expressions and use `alias--language` aliases for multiple languages. The `studio_translate`, `studio_number` and `studio_date` helpers execute compiled translations at runtime. Automatic platform language selection remains deferred.

Imports validate the entire bundle before database writes. They reject traversal/absolute paths, backslashes, URL escapes, ambiguous names, case collisions, links, unsupported versions/compression, undeclared/missing files and repeated references. Nothing is extracted to disk. Windmill uses the tenant selected in the application rather than the manifest's tenant ID; duplicate aliases after this binding are rejected. Imported records receive normal database creation/update timestamps. Legacy CSV is available through the renderer's migration operations, rather than the platform's ZIP importer.

## Limits

| Item | Limit |
| --- | --- |
| Attached files per template | 100 |
| File size / total attached bytes | 2 MB / 8 MB |
| Relative file path | 240 UTF-8 bytes; no empty or dot segments, reserved document path, or `__proto__` segments |
| Source | 300 KB per template |
| ZIP | 20 MB compressed, 32 MB expanded, 512 entries, 64 templates |
| Native file rendering | 8 MB input HTML plus bounded files; 8 MB PDF; 20 seconds after browser launch |

Repeated files in localized templates count toward the bundle limits. Stored and Deflate ZIP entries are supported. Native rendering uses the existing container Chromium installation; `CHROME` may select the executable. The process deadline requires Unix `kill`, as provided by the report containers. Deployment containers remain responsible for Chromium process isolation.

The versioned catalog contains wrappers, structural data contracts and synthetic fixtures for eight report types. The crate README documents builds; the private Beyond Studio documentation covers editing, deployment, API/MCP and preview limits.
