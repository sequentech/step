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

### Lambda input

The input to the Lambda, is a JSON file of the form:

```
{
  "html": "raw html with escaped quotes",
  "pdf_options": {
    "landscape": <bool>,
    "displayHeaderFooter": <bool>,
    "printBackground": <bool>,
    "scale": <float>,
    "paperWidth": <float>,
    "paperHeight": <float>,
    "marginTop": <float>,
    "marginBottom": <float>,
    "marginLeft": <float>,
    "marginRight": <float>,
    "pageRanges": <string>,
    "ignoreInvalidPageRanges": <bool>,
    "headerTemplate": <string>,
    "footerTemplate": <string>,
    "preferCssPageSize": <bool>,
    "transferMode": {
      "mode": <string>
    }
  },
  "bucket": <string>,
  "bucket_path": <string>
}
```

**All keys are optional, except for `html`.**

### Backends

#### `aws_lambda`

The environment variable that ponits to the AWS Lambda endpoint is
`AWS_LAMBDA_DOC_RENDERER_ENDPOINT`. It has not a default value, so
that the PDF generation will fail if it is missing.

**In the AWS Lambda mode, if the Lambda is provided with a bucket, it
will try to upload the generated PDF to S3 at the provided path and S3
bucket. If the upload to S3 fails, the generation of the PDF as a
whole will return failure, as if it was never generated.**

#### Test

First create the Lambda, assume we got as Function URL
`https://rq5jtxuv4rxo5viu5jmxmpxuqq0oisgh.lambda-url.us-east-1.on.aws/`
in this example.

In a terminal, go to `step`, run `devenv shell`. Now, `cd
packages/sequent-core`. From this location, run:

```
step/packages/sequent-core $ AWS_LAMBDA_DOC_RENDERER_ENDPOINT=https://rq5jtxuv4rxo5viu5jmxmpxuqq0oisgh.lambda-url.us-east-1.on.aws/ cargo run -q --features=reports,lambda_aws_lambda --example render_pdf
PDF correctly generated. Lambda is working as expected.
```

If you see the message `PDF correctly generated. Lambda is working as
expected.`, the Lambda is accessible and reporting a valid response.

##### Testing lambda with curl

You can also test the lambda manually with curl, like so:

```
$ curl -H 'Content-Type: application/json' \
    -d '{ "html": "Hello, world" }' \
    https://rq5jtxuv4rxo5viu5jmxmpxuqq0oisgh.lambda-url.us-east-1.on.aws/ | jq
```

#### `inplace`

**This mode is not relevant in production mode.**

No extra envvars are relevant other than `PATH`, or `CHROME` if you
want to point explicitly to a specific Chrome executable.

#### `openwhisk`

**This mode is not relevant in production mode.**

The environment variable that points to the OpenWhisk endpoint is
called `OPENWHISK_DOC_RENDERER_ENDPOINT`. If its value is not provided, it will be
defaulted to `http://$OPENWHISK_API_HOST:3233/api/v1/namespaces/_/actions/pdf-tools/doc_renderer?blocking=true&result=true`.

### Services to update with the new environment variables

Environment variables to add, in production:

- `DOC_RENDERER_BACKEND=aws_lambda`
- `AWS_LAMBDA_DOC_RENDERER_ENDPOINT=<endpoint>`: this endpoint should
  be the Function URL, with **NONE** **Auth type**. The requests that
  are issued by `sequent-core` to the Lamdba **are not IAM
  authenticated at this time**. It is of the form
  `https://rq5jtxuv4rxo5viu5jmxmpxuqq0oisgh.lambda-url.us-east-1.on.aws/`
  (**DO NOT USE THIS ONE IN PARTICULAR, AS IT WILL PROBABLY NOT
  EXIST.**)
- `AWS_S3_PRIVATE_URI`
- `AWS_S3_PUBLIC_URI`
- `AWS_S3_BUCKET`
- `AWS_S3_PUBLIC_BUCKET`
- `AWS_REGION`
- `AWS_S3_ACCESS_KEY`
- `AWS_S3_ACCESS_SECRET`
- `AWS_S3_MAX_UPLOAD_BYTES`
- `AWS_S3_UPLOAD_EXPIRATION_SECS`
- `AWS_S3_FETCH_EXPIRATION_SECS`

Impacted services:

- `windmill`
- `harvest`

As a rule of thumb, **anything** that calls to `render_pdf` from
`packages/sequent-core/src/services/pdf.rs` will have this
behavior. This **applies transitively** across dependencies.
