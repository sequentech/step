<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# orare threat model

orare has two parts. The `orare` crate (`src/lib.rs`) is a procedural macro, `lambda_runtime`, that wraps an async function into an AWS Lambda function URL handler (feature `aws_lambda`) or into a command-line `main` that reads JSON from its first argument (feature `openwhisk`); the macro itself runs only at compile time. The `doc_renderer` crate (`doc_renderer/`) is its only consumer: an HTML-to-PDF service shipped as a container image (`doc_renderer/Dockerfile`, Alpine with Chromium) that drives headless Chromium through sequent-core's PDF service. Built with `aws_lambda`, it runs on AWS Lambda, reads the HTML from S3 and writes the PDF back to S3. Built with `openwhisk` (the default feature), it is a warp HTTP server (`doc_renderer/src/openwhisk.rs`) that OpenWhisk runs as a Docker action; this variant does not use the macro. windmill (reports, stored HTML documents, post-tally reports) and velvet (ballot images, generated reports) reach it through sequent-core `PdfRenderer` when `DOC_RENDERER_BACKEND` is `aws_lambda` or `openwhisk`; with `inplace`, the default in the example environments, Chromium runs inside the caller and doc_renderer is not used. Its input can carry voter PII, credentials and decoded ballots, and its output is the PDF that election managers publish or hand to voters. See the [system threat model](../../THREAT_MODEL.md).

## Assets

- **HTML documents in transit and at rest**: rendered templates carrying voter PII and credentials, decoded ballots and results. Confidentiality.
- **Rendered PDFs**: official reports, results, receipts and letters. Integrity; confidentiality for per-voter documents.
- **Object storage access** (Lambda variant): the S3 credentials or Lambda role it uses, and the objects it reads and writes. Confidentiality of the credentials, integrity of the objects.
- **Renderer runtime**: the Chromium process and the container or Lambda environment it runs in. Integrity: whoever controls it controls every PDF it produces.
- **Bundled assets** (`doc_renderer/assets/`): `qrcode.min.js` runs in every page that loads it; logos and backgrounds appear in official documents. Integrity.
- **Renderer availability**: report generation in windmill and velvet waits on it.

## Entry points and trust boundaries

| Entry point | Who can reach it | Trust | Checks in place |
|---|---|---|---|
| `src/lib.rs` `lambda_runtime` | the Rust compiler, at build time | developer | parses the annotated function as `syn::ItemFn`; no runtime input |
| Generated Lambda handler `func` wrapping `doc_renderer/src/main.rs` `render_pdf` (feature `aws_lambda`) | windmill and velvet, through the Lambda function URL | internal service | body deserialized into `doc_renderer/src/io.rs` `Input`; `Input::Raw` rejected; access control by the deployment |
| S3 object holding the input HTML (`Input::S3`) | whoever can write to the bucket | internal service | must be valid UTF-8 (`String::from_utf8`) |
| `doc_renderer/src/openwhisk.rs` `start_server` route `POST /run` (`OpenWhiskInput.value`) | the OpenWhisk invoker, and anything else that can reach the action container | internal service | body deserialized into `OpenWhiskInput`; `Input::S3` rejected in `handle_render_impl`; access control by the deployment |
| `pdf_options` in either input (`PrintToPdfOptions`, including `header_template` and `footer_template`) | same as the request carrying it | internal service | passed to Chromium as given |
| `doc_renderer/src/openwhisk.rs` `POST /init`, `GET /health` | same as `/run` | internal service | take no input |
| Generated command-line `main` (feature `openwhisk`) in `src/lib.rs` | process launcher | operator | `deserialize_str` into the function's `Input`; not used by `doc_renderer` |
| Image build arguments and environment (`doc_renderer/Dockerfile`) | image builder; Lambda or OpenWhisk configuration | operator | set by the operator |
| `/assets/` files in the image (`doc_renderer/Dockerfile` `ADD ./orare/doc_renderer/assets`) | templates that load `/assets/qrcode.min.js` and images | repository content | versioned in the repository |

## Threats

| ID | STRIDE | Threat | Controls in the code | Status |
|---|---|---|---|---|
| orare-T1 | Spoofing | A party other than windmill or velvet invokes the renderer. | Access control by the deployment (Lambda function URL, OpenWhisk API). Each build accepts one input form (`doc_renderer/src/main.rs` `render_pdf` rejects `Input::Raw`, `doc_renderer/src/openwhisk.rs` `handle_render_impl` rejects `Input::S3`). | Not verified |
| orare-T2 | Elevation of privilege | Crafted HTML exploits the headless browser and runs code in the renderer. | Handlebars escapes `{{ }}` by default and registers a `sanitize_html` helper (`../sequent-core/src/services/reports.rs` `get_registry`). With these backends the browser runs in its own container, apart from windmill and velvet. | Partial |
| orare-T3 | Information disclosure | Rendered HTML makes the browser read local files or internal endpoints and leak them into the PDF. | Network isolation (deployment). | Not verified |
| orare-T4 | Information disclosure | Sensitive document content (voter credentials, PII, decoded ballots) is retained or leaked on the renderer path. | The temporary HTML file is removed when the `tempdir` is dropped. | Partial |
| orare-T5 | Tampering | Rendered PDFs are tampered with in storage. | The S3 transport uses the private bucket; who can write to it is set by the storage configuration (deployment). | Partial |
| orare-T6 | Information disclosure | Storage credentials leak from the image or the renderer. | `doc_renderer/Dockerfile` defaults the `AWS_S3_*` arguments to empty and the dev build does not set them; sequent-core `build_s3_aws_config_for_endpoint` then uses the default credential chain (the Lambda role). | Partial |
| orare-T7 | Denial of service | Oversized or slow requests tie up the renderer and stall report generation. | Lambda and OpenWhisk platform limits (deployment). | Partial |
| orare-T8 | Denial of service | A malformed request crashes the handler. | The generated Lambda handler in `src/lib.rs` panics on bad input, which fails that invocation; warp rejects `/run` bodies that do not deserialize into `OpenWhiskInput`. | Accepted (by design: a bad request fails only its own invocation; the platform keeps serving others) |
| orare-T9 | Tampering | A tampered image, bundled script or dependency changes every rendered document. | `BASE_IMAGE` defaults to a digest-pinned Alpine image (`doc_renderer/Dockerfile`, also passed by the dev compose files); `qrcode.min.js` is vendored in `doc_renderer/assets/`; `auto_generate_cdp` is patched to a fixed git revision in `doc_renderer/.cargo/config.toml`. | Partial |
| orare-T10 | Repudiation | Nobody can tell who asked for which document to be rendered. | The renderer is stateless and keeps no record of requests. | Accepted (by design: accountability for document generation sits with the caller, windmill or velvet) |

## Assumptions

- windmill and velvet are the only callers, and they escape untrusted values in templates.
- Each deployment restricts who can invoke the renderer and gives it least-privilege storage credentials.
- The OpenWhisk deployment protects its API credentials and lets only the invoker reach action containers.
- The deployment limits the renderer's network egress.
- Operators build, patch and deploy the image through a trusted pipeline.
- Traffic between windmill, velvet, the renderer and object storage uses TLS where it crosses an untrusted network.

## Review focus

1. Isolation of the headless browser and what a compromised renderer can reach.
2. Access control for both deployment variants.
3. Handling of sensitive document content on the renderer path.
4. Resource limits on rendering.
5. Image build and provenance.
6. Error handling in the code generated by `src/lib.rs`, if more functions adopt the macro.
