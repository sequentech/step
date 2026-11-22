#!/bin/bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequent.io>
# SPDX-License-Identifier: AGPL-3.0-only
set -euo pipefail

export AWS_ACCESS_KEY_ID=test AWS_SECRET_ACCESS_KEY=test AWS_DEFAULT_REGION=us-east-1
s3() { awslocal --endpoint-url=http://127.0.0.1:4566 s3api "$@"; }
bucket=wbraid-messages
if ! s3 head-bucket --bucket "$bucket" 2>/dev/null; then
    # A simultaneous local initializer may have just created the bucket.
    s3 create-bucket --bucket "$bucket" >/dev/null || s3 head-bucket --bucket "$bucket"
fi
s3 put-bucket-cors --bucket "$bucket" --cors-configuration file:///aws/s3-cors.json
