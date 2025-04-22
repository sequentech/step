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

## Airgap mode and the VSTL process

Assuming the VSTL process as follows:

- Team A:
    - Has Internet access
        - Lacks access to our Amazon ECR instance
    - Builds the initial airgap environment
    - Downloads all project dependencies
- Team B:
    - Lacks Internet access
    - Retrieves images created by Team A
    - Builds the project
- Team C:
    - Might lack Internet access
    - Retrieves images created by Team B
    - Runs the project

### Team A

First download all dependencies for build airgapped in offline mode, to do this, choose a version and launch those commands

```
git checkout v8.0.7
STEP_VERSION=v8.0.7 STEP_HASH=4588af5d44 scripts/build-airgapped.sh
```

This creates a file like /step/airgapped-artifacts/YYYY-MM-DD.tar. It prints some warnings for missing images, specially those coming from 581718213778.dkr.ecr.us-east-1.amazonaws.com. This is not relevant, as they will be produced later on by Team B.

### Team B

0. They get the artifacts from Team A.

0. They build the project:

    ```
    sudo ./offline_trusted_build.sh 
    ```

0. They retag some images:

    ```
    export TAG=<tag>
    export DESIRED_PREFIX="581718213778.dkr.ecr.us-east-1.amazonaws.com"
    for component in harvest windmill admin-portal braid voting-portal b3 immudb-init keycloak immudb; do
        docker tag ${component}-build:latest $DESIRED_PREFIX/$component:$TAG
    done
    ```

0. They extract the tar provided by Team A that contains some of the
   offline artifacts:

    ```
    $ mkdir out
    $ tar -xf YYYY-MM-DD.tar -C out
    ```

0. They add the retagged images to the tarball:

    ```
    export TAG=<tag>
    export DESIRED_PREFIX="581718213778.dkr.ecr.us-east-1.amazonaws.com"
    for component in harvest windmill admin-portal braid voting-portal b3 immudb-init keycloak immudb; do
        docker save $DESIRED_PREFIX/$component:$TAG > out/images/$component:$TAG.tar
    done
    ```

0. They re-create the tarball with all images, the base ones provided
by Team A, and the ones built by Team B from the source code, and
retagged:

    ```
    tar -cvzf out-final.tar.gz out
    ```

### Team C

0. They get the artifacts from Team B.
0. They follow the instructions in the README.md in the airgap artifact.
0. After all preconditions in that file are met, as documented in that
   file, they can run:

    ```
    sudo su -
    $ ./up <trustees_ezip> <password> <excel_path>
    ```
