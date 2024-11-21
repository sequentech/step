<!--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# doc_renderer Lambda Function

This project is a Rust-based lambda function that can be executed in different
environments, such as **AWS Lambda** or **OpenWhisk**. It uses the `orare`
library, which provides shared functionality and abstracts away
platform-specific details using feature flags.

## Features

- **AWS Lambda**: Runs in AWS Lambda using the `lambda_runtime`.
- **OpenWhisk**: Runs in an OpenWhisk-compatible environment by simulating
  command-line input and output.
- **Shared Code**: The `orare` library contains shared business logic,
  input/output definitions, and procedural macros.

## Build Instructions

1. Make sure you are in the `doc_renderer` directory before running the following
commands.
2. Temporarily, install the following to your development environment:
```bash
sudo apt-get install software-properties-common
sudo add-apt-repository universe
sudo apt-get update
sudo apt-get install wkhtmltopdf
```
### Build and Run for InPlace

To build and run the `doc_renderer` lambda function with the **InPlace**
feature, you can user `cargo run` with the `inplace` feature enabled:

```bash
RUST_LOG=debug cargo run --features inplace -- '{"html": "<h1>Test InPlace Rendering</h1>", "pdf_options": null}'
```

This will generate a file and you can view it from the terminal.

### Build and Run for OpenWhisk

To build and run the `doc_renderer` lambda function with the **OpenWhisk**
feature, you can use `cargo run` with the `openwhisk` feature enabled:

```bash
cargo run --features openwhisk -- '{"name": "OpenWhisk"}'
```

This will run the lambda function with the `openwhisk` feature, passing
`{"name": "OpenWhisk"}` as input, and the following output is expected:

```json
{"message": "Hello, world!"}
```
