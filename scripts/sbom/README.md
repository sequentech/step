<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Evidence-based SBOM preparation

This generator prepares the OVCS source inventory and printable rights schedules from frozen Syft scans, source checkouts and exact-release license evidence. It does not claim to scan deployed images or establish unsigned vendor rights.

Use Python 3.12 or later (`tomllib` and safe tar extraction are required). Install Lato fonts at `/usr/share/fonts/truetype/lato/Lato-*.ttf` (Debian/Ubuntu: `fonts-lato`). Install Python dependencies: reportlab, openpyxl, pypdf, license-expression, pyyaml. Run:

```sh
SBOM_EVIDENCE_DIR=/path/to/inputs \
SBOM_SOURCE_DIR=/path/to/checkouts \
SBOM_OUTPUT_DIR=/path/to/output \
python scripts/sbom/generate.py
```

Inputs contain `metadata-cache.json`, `step.syft.json`, `step-maven.syft.json`, `beyond.syft.json`, `beyond-maven.syft.json`, and `licenses/*.json` (exact artifact URL, SHA-256 and license text). Checkouts contain `step/` and `beyond/`. Use scans matching the checked-out source; metadata-only SPDX changes may reuse a scan with that fact recorded. The generated source revisions identify the reviewed checkouts. The source/deployment distinction is deliberate:

- Concrete Dockerfile/YAML image references go into Infrastructure-References.csv. SQL, prose, templates and build-stage aliases must not become software packages.
- Unversioned Maven `provided` dependencies remain in Provided-Platform-Dependencies.csv until their supplying image is scanned. This file is included per repository in the ZIP and SHA256SUMS.
- Maven test scope, explicitly excluded tools, and evidenced optional transitive dependencies are logged in the internal exclusions ledger.
- Beyond module attribution reads an isolated archive of its committed Step gitlink, including third-party notices. That commit must exist in the local Step object database; a missing object aborts generation rather than borrowing another revision’s rights. The source checkouts are not switched or modified. Both gitlinks are recorded in provenance JSON.
- Exact artifact notices can correct scanner declarations; raw evidence remains available. Existing confirmation flags remain open for explicit review, even after a license declaration is corrected. A license entry does not establish fulfillment of all redistribution obligations.

Before submission, add the final image/OS SBOMs and manifests, reconcile selected modules and accounts, preserve applicable notices/source delivery, and execute the vendor/bidder grants. Zero unknown source-license declarations does not close these tasks.

Validation: `python -m unittest discover -s scripts/sbom -p 'test_*.py'`; inspect the generated SPDX files with spdx-tools, validate all PDFs, and extract/hash-compare the final ZIP. Meta issue: https://github.com/sequentech/meta/issues/14601.
