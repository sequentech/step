<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->
# Shared template renderer

Pure Handlebars HTML generation extracted from `sequent-core::services::reports`. Production callers retain that re-export. The `template_rendering` feature exposes it without PDF/network dependencies; the production `reports` feature includes it. This crate has no database, networking, Chromium or private repository dependency. A standalone Cargo workspace keeps browser/server WASM builds independent of platform services and native crypto tooling.

`studio_execute(JSON) -> JSON` and native `execute(Value) -> Value` support `catalog`, `render`, `import_zip`, `export_zip` and `validate_assets`. The old `import`/`export` CSV operations remain available for migration. The version 1 catalog contains exactly the eight production report types, their user/system templates, configuration, offline structural schemas and synthetic scenarios. The templates and initial fixtures are snapshots of `.devcontainer/minio/public-assets`; updates must deliberately update the versioned catalog. Electoral-results variants cover small elections, multiple contests, many candidates, long names, zero turnout and optional data absence.

Rendering composes user HTML into `rendered_user_template` in the system wrapper. Existing helpers remain unchanged; additional `studio_translate`, `studio_number` and `studio_date` helpers support compiled multilingual exports. Authoring helpers `t`, `number` and `date` resolve exact → base → default language catalogs without replacing runtime data expressions. Plural rules use CLDR data through `intl_pluralrules`; dates use deterministic input calendar dates, and decimal formatting uses locale separators.

The ZIP codec is shared directly with Windmill's import/export tasks. A versioned manifest points to UTF-8 Handlebars sources and real binary files; ordinary template settings and typed metadata remain JSON. Imports validate every path and reference and never extract an archive to the filesystem. Windmill binds imported records to the selected tenant. See [the bundle format and rendering contract](../../docs/docusaurus/docs/05-reference/template-renderer.md).

A template's optional `assets` map stores relative paths with `{ "mime": "…", "base64": "…" }` values in its existing JSON column. The pure renderer validates it and carries it in an inert HTML comment envelope. Native PDF transports consume the envelope in `sequent-core::services::template_pdf`; browser clients must not enable scripts in an authenticated application iframe. The legacy CSV decoder remains available for migration, but the platform's template import/export boundary is now ZIP.

```sh
cargo test --locked --manifest-path packages/template-renderer/Cargo.toml
cargo build --locked --manifest-path packages/template-renderer/Cargo.toml --release --target wasm32-unknown-unknown
wasm-bindgen --target web --out-dir /tmp/studio-web packages/template-renderer/target/wasm32-unknown-unknown/release/sequent_template_renderer.wasm
wasm-bindgen --target nodejs --out-dir /tmp/studio-node packages/template-renderer/target/wasm32-unknown-unknown/release/sequent_template_renderer.wasm
cargo run --locked --manifest-path packages/template-renderer/Cargo.toml --example parity
```

Use wasm-bindgen **0.2.104**. Beyond's companion package compares native parity vectors with both WASM targets and runs real PostgreSQL/MCP/browser tests. Inputs and HTML output are bounded; consumers run untrusted renders in workers with deadlines. HTML is untrusted: consumers must sandbox it and restrict network/script execution. The optional `template_pdf` feature exposes isolated native file rendering without the production AWS dependencies.

Related: https://github.com/sequentech/meta/issues/13383. Based on Step #3087; the private Studio implementation is in Beyond.
