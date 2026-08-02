---
id: cli-tutorials-generate-preview-url
title: Generating a Preview URL for a Ballot Style
position: 5
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

This guide walks you through generating a preview URL from a ballot style document using the Step CLI.

---

## Prerequisites 
Before you begin, ensure that you have a **valid** ballot style JSON file available.
The file must exist at the path specified by the `--file-path` argument in the following `generate-preview-url` command.

---

## Step 1: Authenticate as a Admin
Configure the CLI to point to the correct tenant and Keycloak instance, and authenticate as an ***admin user***:
```bash
cd packages/step-cli
cli step config --tenant-id 90505c8a-23a9-4cdf-a26b-4e19f6a097d5 \
--endpoint-url http://graphql-engine:8080/v1/graphql \
--keycloak-url http://keycloak:8090 \
--keycloak-user admin \
--keycloak-password admin \
--keycloak-client-id api-key-client \
--keycloak-client-secret 4lzmxNgZHjfzS5BwDVlyrRUDqwvFLUvL
```

## 2. Generate the Preview URL
Once authenticated, generate the preview URL by providing a path to a valid ballot style JSON file:
```bash
cli step generate-preview-url \
--file-path ./data/afdda2b9-014d-46b0-a32c-aa6526918f23.json
```

### Output:
On success, the command will return a preview URL.
