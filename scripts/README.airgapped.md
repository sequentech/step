<!--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Offline mode

From within the root of the `step` git project, you can build the artifacts by
running:

```bash
export STEP_VERSION=v4.0.1-rc.2
export STEP_HASH=ee85db6
./scripts/build-airgapped.sh
```

This will produce a `<date>.tar.xz` file under `airgapped-artifacts` folder on
the root of the project. This is the tarball that contains the `./up` script
along with the docker images and everything required to run Sequent Step
platform offline.
