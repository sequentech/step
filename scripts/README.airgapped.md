<!--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Offline mode

## Docker Engine

Install and configure [Docker Engine](https://docs.docker.com/engine/install/ubuntu/). In ubuntu you might have to run it as root.

## AWS config

You need aws access to the ECR. Configure your AWS credentials in `~/.aws/credentials` with something like:

```
[default]
aws_access_key_id = <key>
aws_secret_access_key = <pass>
```

## Github Config

Also you need to access the Github Container Registry. Create a Personal Access Token (Classic) with the permission `read:packages` and set it in your env:

```
export GH_USER=<user>
export GH_PASS=<ghp_password>
```

## yq

Ensure you have yq installed:

```
apt install -y yq
```

## Run  the script

From within the root of the `step` git project, you can build the artifacts by
running:

```bash
set -a
source .devcontainer/.env
aws ecr get-login-password --region us-east-1 | docker login --username AWS --password-stdin 581718213778.dkr.ecr.us-east-1.amazonaws.com
docker login ghcr.io -u $GH_USER -p $GH_PASS
STEP_VERSION=v6.2.1-rc.2 STEP_HASH=bd29c29 scripts/build-airgapped.sh
```

This will produce a `<date>.tar.xz` file under `airgapped-artifacts` folder on
the root of the project. This is the tarball that contains the `./up` script
along with the docker images and everything required to run Sequent Step
platform offline.
