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
❯ cargo run --features openwhisk -- '{"name": "OpenWhisk"}'
```

#### Building the lambda container image

In order to create the lambda container image, run, from the
`/packages` directory:

```bash
❯ docker build --push -f orare/doc_renderer/Dockerfile -t <someuser>/doc_renderer:latest .
```

#### Creating the lambda container image in OpenWhisk

Although optional, first create the package:


```bash
❯ openwhisk-cli package create pdf-tools
ok: created package pdf-tools
```

Now, create the action:

```bash
❯ openwhisk-cli action create pdf-tools/doc_renderer \
    --web no \
    --docker <someuser>/doc_renderer:latest
```

Note that in order for the action to be active, the OpenWhisk
container needs to be able to pull the image (it's not enough for our
Docker host --used by OpenWhisk through the Docker socket-- to have
rights to pull from the container image registry.)

Now you can invoke the action:

```bash
❯ openwhisk-cli action invoke pdf-tools/doc_renderer --blocking --result -P <(echo '{"html":"hi"}')
{
    "pdf_base64": "JVBER..."
}
```
