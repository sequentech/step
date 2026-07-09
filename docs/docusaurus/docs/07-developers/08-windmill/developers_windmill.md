---
id: developers_windmill
title: Developers Windmill
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## Default Keycloak realm templates

Windmill loads the default tenant and election-event Keycloak realm templates
from the private S3-compatible object store. The object keys are configured with
the following environment variables and are relative to `AWS_S3_BUCKET`:

| Variable | Development value |
| --- | --- |
| `KEYCLOAK_TENANT_REALM_CONFIG_S3_KEY` | `defaults/keycloak/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json` |
| `KEYCLOAK_ELECTION_EVENT_REALM_CONFIG_S3_KEY` | `defaults/keycloak/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5-event-33f18502-a67c-4853-8333-a58630663559.json` |

Keys must not start with `/`. The files in `.devcontainer/keycloak/import/`
remain the source of truth. In development, remote-compose, and air-gapped
deployments, `configure-minio` uploads both files to the private bucket before
Windmill starts. It overwrites the objects each time it runs, so the
`logs.reset.minio` VS Code task re-provisions templates after local edits.
Keycloak's own startup realm import continues to read the local files directly.

### Production provisioning

Upload both templates before deploying a Windmill version that uses the S3 key
variables. `AWS_S3_BUCKET` can be a logical prefix inside the physical private
AWS bucket. Set `PRIVATE_PREFIX` to that prefix, or leave it empty when the
deployment does not use one.
Never upload these realm templates to the public bucket.

```bash
export PRIVATE_BUCKET="<physical-private-bucket>"
export PRIVATE_PREFIX="<logical-private-prefix>"
export KEYCLOAK_TENANT_REALM_CONFIG_S3_KEY="defaults/keycloak/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json"
export KEYCLOAK_ELECTION_EVENT_REALM_CONFIG_S3_KEY="defaults/keycloak/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5-event-33f18502-a67c-4853-8333-a58630663559.json"

PRIVATE_ROOT="s3://${PRIVATE_BUCKET}"
if [ -n "$PRIVATE_PREFIX" ]; then
  PRIVATE_ROOT="${PRIVATE_ROOT}/${PRIVATE_PREFIX}"
fi

aws s3 cp \
  .devcontainer/keycloak/import/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json \
  "${PRIVATE_ROOT}/${KEYCLOAK_TENANT_REALM_CONFIG_S3_KEY}"
aws s3 cp \
  .devcontainer/keycloak/import/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5-event-33f18502-a67c-4853-8333-a58630663559.json \
  "${PRIVATE_ROOT}/${KEYCLOAK_ELECTION_EVENT_REALM_CONFIG_S3_KEY}"
```

Verify each upload against the physical bucket. The object key is the logical
prefix, when present, followed by the configured realm key.

```bash
TENANT_OBJECT_KEY="${KEYCLOAK_TENANT_REALM_CONFIG_S3_KEY}"
EVENT_OBJECT_KEY="${KEYCLOAK_ELECTION_EVENT_REALM_CONFIG_S3_KEY}"
if [ -n "$PRIVATE_PREFIX" ]; then
  TENANT_OBJECT_KEY="${PRIVATE_PREFIX}/${TENANT_OBJECT_KEY}"
  EVENT_OBJECT_KEY="${PRIVATE_PREFIX}/${EVENT_OBJECT_KEY}"
fi

aws s3api head-object --bucket "$PRIVATE_BUCKET" --key "$TENANT_OBJECT_KEY"
aws s3api head-object --bucket "$PRIVATE_BUCKET" --key "$EVENT_OBJECT_KEY"
```

For an existing deployment, replace
`KEYCLOAK_TENANT_REALM_CONFIG_PATH` and
`KEYCLOAK_ELECTION_EVENT_REALM_CONFIG_PATH` with the corresponding `*_S3_KEY`
variables. Upload the objects first, then deploy the Step, beyond, and gitops
configuration changes together. The Windmill error for a missing object names
the configured bucket and key.

## Tally

### Discarded/Auditable Ballots

Discarded Ballots that for some reason are not included in the tally. Possible reasons:

- The voter was disabled.
- The voter was deleted.
- The ballot was cast outside the Voting Period.
- The voter is not authorized for the election of the ballot.
- The voter is not assigned to the area of the ballot.
- The ballot is from a previous revote (only the last vote counts). (Note, this is not counted as a discarded ballot yet).

### Eligible Voters

Eligible voters are voters that can vote. Voters not included here are disabled and deleted voters.
