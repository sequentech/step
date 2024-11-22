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
