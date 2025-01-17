<!--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Release NEXT

## ✨ Remove tagalo from admin settings in janitor

In order to ensure that tagalo is not active as a language in the admin portal, ensure
that in the excel file you're using for janitor, you have this configuration: in the
`Parameters` tab, add a row with:

- type: admin
- key: tenant_configurations.settings.language_conf.enabled_language_codes
- value: ["en"]

## ✨ Move all PDF generation to Lambda functions

PDF generation backend is now configurable and allows Lambdas to
perform this action. Generated PDF's might also be pushed to S3 --if
the Lambda backend is configured as AWS.--

The **environment variable** that mandates which PDF renderer backend
to use is called `DOC_RENDERER_BACKEND`.

There are three renderers allowed:

- `DOC_RENDERER_BACKEND=aws_lambda`: this uses the AWS Lambda service -- only through a
  Function URL in with `NONE` `Auth type`
  (https://docs.aws.amazon.com/lambda/latest/dg/urls-configuration.html). This
  backend is used in **production.**
- `DOC_RENDERER_BACKEND=inplace`: this forks chrome/chromium,
  generating the PDF in place. This is how PDF's have been generated
  in the past. This is still the default in the development
  environment, until `openwhisk` is promoted to the default in this
  environment.
- `DOC_RENDERER_BACKEND=openwhisk`: this service can be started in
  development mode (`.devcontainer/docker-compose.yaml`), and the PDF renderer
  lambda will be built and served locally in this mode.

Depending on the chosen renderer, other environment variables might be
relevant. **Note that this environment variables, both
`DOC_RENDERER_BACKEND` and the ones following based on the backend
that was chosen will need to be configured on multiple services, as
the renderer decision that reads the `DOC_RENDERER_BACKEND` is inside
`sequent-core`.**

### Backends

#### `aws_lambda`

The environment variable that ponits to the AWS Lambda endpoint is
`AWS_LAMBDA_ENDPOINT`. It has not a default value, so that the PDF
generation will fail if it is missing.

#### Test

First create the Lambda, assume we got as Function URL
`https://rq5jtxuv4rxo5viu5jmxmpxuqq0oisgh.lambda-url.us-east-1.on.aws/`
in this example.

In a terminal, go to `step`, run `devenv shell`. Now, `cd
packages/sequent-core`. From this location, run:

```
step/packages/sequent-core $ AWS_LAMBDA_ENDPOINT=https://rq5jtxuv4rxo5viu5jmxmpxuqq0oisgh.lambda-url.us-east-1.on.aws/ cargo run -q --features=reports,lambda_aws_lambda --example render_pdf
PDF correctly generated. Lambda is working as expected.
```

If you see the message `PDF correctly generated. Lambda is working as
expected.`, the Lambda is accessible and reporting a valid response.

#### `inplace`

**This mode is not relevant in production mode.**

No extra envvars are relevant other than `PATH`, or `CHROME` if you
want to point explicitly to a specific Chrome executable.

#### `openwhisk`

**This mode is not relevant in production mode.**

The environment variable that points to the OpenWhisk endpoint is
called `OPENWHISK_ENDPOINT`. If its value is not provided, it will be
defaulted to
`http://127.0.0.2:3233/api/v1/namespaces/_/actions/pdf-tools/doc_renderer?blocking=true&result=true`.

### Services to update with the new environment variables

Environment variables to add, in production:

- `DOC_RENDERER_BACKEND=aws_lambda`
- `AWS_LAMBDA_ENDPOINT=<endpoint>`: this endpoint should be the
  Function URL, with **NONE** **Auth type**. The requests that are
  issued by `sequent-core` to the Lamdba **are not IAM authenticated
  at this time**. It is of the form
  `https://rq5jtxuv4rxo5viu5jmxmpxuqq0oisgh.lambda-url.us-east-1.on.aws/`
