<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->
# Template rendering

`packages/template-renderer` owns the pure Handlebars renderer used by native report generation and Template Studio's browser/server WASM bindings. `sequent-core::services::reports` re-exports its existing functions so Windmill and Velvet retain production helper and wrapper behavior. Chromium remains part of optional PDF generation, not HTML rendering.

The versioned catalog provides wrappers, structural data contracts and synthetic fixtures for the eight supported report types. Native tests cover localization and ordinary template CSV compatibility; the same CSV decoder is used by Windmill's import task. See the crate README for build commands and the private Beyond Template Studio documentation for authoring and deployment.

Localized CSV exports preserve runtime expressions and use ordinary template fields. The additional `studio_translate`, `studio_number` and `studio_date` helpers execute compiled translations at runtime. Automatic language selection in Admin Portal and Windmill is deferred; operators select the explicitly exported language alias.
