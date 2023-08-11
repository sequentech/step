<!--
SPDX-FileCopyrightText: 2022 David Ruescas <david@sequentech.io>
SPDX-FileCopyrightText: 2022-2023 Eduardo Robles <edu@nsequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->
# bulletin-board
[![Discord][discord-badge]][discord-link]
[![Build Status][build-badge]][build-link]
[![codecov][codecov-badge]][codecov-link]
[![Dependency status][dependencies-badge]][dependencies-link]
[![License][license-badge]][license-link]
[![REUSE][reuse-badge]][reuse-link]

[Documentation]

Sequent Bulletin Board. This is a work-in-progress - not usable yet. Implemented
using Rust and [trillian].

## How-to

Launch this project using [Github dev containers]. Then, in a terminal, you
can launch the server with:

<details>
  <summary>Click to expand</summary>

```bash
# create temporal directory to work with and print it for later
export STORAGE_DIR=$(mktemp -d)
echo "export STORAGE_DIR=$STORAGE_DIR"

# execute the commands in debug mode
export TRACING_LEVEL=debug

# create the client key pair and assign it to the admin user
cargo run --bin create-key > ${STORAGE_DIR}/client-keys.toml
export PUBLIC_KEY=$(tomlq .public_key ${STORAGE_DIR}/client-keys.toml)

# create the service config
cat << EOF > ${STORAGE_DIR}/service-config.toml
storage_path = "${STORAGE_DIR}"
server_url = "127.0.0.1:3000"
[[permissions.users]]
name = "admin"
public_key = ${PUBLIC_KEY}
metadata = {}
[[permissions.roles]]
name = "admins"
permissions = [ "CreateBoard" ]
metadata = {}
[[permissions.user_roles]]
user_name = "admin"
role_names = [ "admins" ]
EOF

# run the service
CONFIG_PATH=${STORAGE_DIR}/service-config.toml \
cargo run \
   --features="build-server" \
   --bin bulletin-board-server
```

</details>

Then, in another terminal you can request to create a board with:

<details>
  <summary>Click to expand</summary>

```bash
# Change the storage dir to whatever was printed in the other terminal before
export STORAGE_DIR=/tmp/tmp.puAxZ7RC6x

# retrieve the public key, to be used in the request
export PUBLIC_KEY=$(tomlq .public_key ${STORAGE_DIR}/client-keys.toml)

# generate the unsigned request
cat << EOF > request.json
{
    "board_uuid": "e04bd3c6-1beb-4ef6-9d65-c411ba7c6d08",
    "board_name": "2023-presidential-election",
    "board_description": "",
    "board_metadata": {},
    "signer_public_key": "",
    "signature": "",
    "permissions": {
        "users": [
            {
                "name": "admin",
                "public_key": ${PUBLIC_KEY},
                "metadata": {}
            }
        ],
        "roles": [
            {
                "name": "admins",
                "permissions": [ "AddEntries" ],
                "metadata": {}
            }
        ],
        "user_roles": [
            {
                "user_name": "admin",
                "role_names": [ "admins" ]
            }
        ]
    }
}
EOF

# sign the request and pipe it into the grpcurl API call
cargo run --bin sign -- \
    -p "${STORAGE_DIR}/client-keys.toml" \
    -k 'create-board-request' \
    -d "$(cat request.json)" | \
grpcurl \
    -emit-defaults \
    -plaintext \
    -import-path ./proto \
    -proto bulletin_board.proto \
    -d '@' \
    '127.0.0.1:3000' \
    bulletin_board.BulletinBoard/CreateBoard
```
</details>

This should return something like:

<details>
  <summary>Click to expand</summary>

```json
{
  "bulletinBoard": {
    "uuid": "e04bd3c6-1beb-4ef6-9d65-c411ba7c6d08",
    "name": "2023-presidential-election",
    "description": "",
    "isArchived": false,
    "publicKey": "2023-presidential-election+6f8c9f81+Abwac2R0XtmiAzmDA7/BO5kVzTCA/fhhyK6Uu/j3CPr6",
    "metadata": {
      
    },
    "permissions": {
      "users": [
        {
          "name": "admin",
          "publicKey": "gS2o6mE/9PmDYXHqcqKfyfHSVsoKuEip+olBk3YiQCM",
          "metadata": {
            
          }
        }
      ],
      "roles": [
        {
          "name": "admins",
          "permissions": [
            "AddEntries"
          ],
          "metadata": {
            
          }
        }
      ],
      "userRoles": [
        {
          "userName": "admin",
          "roleNames": [
            "admins"
          ]
        }
      ]
    }
  },
  "checkpoint": {
    "origin": "2023-presidential-election",
    "size": "1",
    "hash": "AJP2tEJGBw5830s/0YL863UYlgfmjf52CBwJr8uH+aw="
  }
}
```
</details>

After that, you can see that the board appear in the Board List, which you
can obtain with:

<details>
  <summary>Click to expand</summary>

```bash
grpcurl \
    -emit-defaults \
    -plaintext \
    -import-path ./proto \
    -proto bulletin_board.proto \
    -d '{}' \
    '127.0.0.1:3000' \
    bulletin_board.BulletinBoard/ListBoards
```
</details>

Resulting something like:

<details>
  <summary>Click to expand</summary>

```json
{
  "boards": [
    {
      "boardLastSequenceId": "0",
      "board": {
        "uuid": "e04bd3c6-1beb-4ef6-9d65-c411ba7c6d08",
        "name": "2023-presidential-election",
        "description": "",
        "isArchived": false,
        "publicKey": "2023-presidential-election+6f8c9f81+Abwac2R0XtmiAzmDA7/BO5kVzTCA/fhhyK6Uu/j3CPr6",
        "metadata": {
          
        },
        "permissions": {
          "users": [
            {
              "name": "admin",
              "publicKey": "gS2o6mE/9PmDYXHqcqKfyfHSVsoKuEip+olBk3YiQCM",
              "metadata": {
                
              }
            }
          ],
          "roles": [
            {
              "name": "admins",
              "permissions": [
                "AddEntries"
              ],
              "metadata": {
                
              }
            }
          ],
          "userRoles": [
            {
              "userName": "admin",
              "roleNames": [
                "admins"
              ]
            }
          ]
        }
      }
    }
  ]
}
```
</details>

You can add an entry with:


<details>
  <summary>Click to expand</summary>

```bash
# Change the storage dir to whatever was printed in the other terminal before
export STORAGE_DIR=/tmp/tmp.puAxZ7RC6x

# Generate the unsigned request
cat << EOF > request.json
{
    "board_uuid": "e04bd3c6-1beb-4ef6-9d65-c411ba7c6d08",
    "entries": [
        {
            "data": "$(echo -n 'Hello Board!' | base64)",
            "metadata": {},
            "signer_public_key": "",
            "signature": ""
        }
    ]
}
EOF

# sign the request and pipe it into the grpcurl API call
cargo run --bin sign -- \
    -p "${STORAGE_DIR}/client-keys.toml" \
    -k 'add-entries-request' \
    -d "$(cat request.json)" | \
grpcurl \
    -emit-defaults \
    -plaintext \
    -import-path ./proto \
    -proto bulletin_board.proto \
    -d '@' \
    '127.0.0.1:3000' \
    bulletin_board.BulletinBoard/AddEntries
```
</details>

Resulting in something like:

<details>
  <summary>Click to expand</summary>

```json
{
  "entries": [
    {
      "sequenceId": "1",
      "entryData": {
        
      },
      "metadata": {
        
      },
      "signerPublicKey": "4wGsjyIMJHUsGjqnoIKEGD6m40bY1zhc7Jh3ob277+s",
      "signature": "pHUrPXrvPGOmqv8Apc3exoNqnFbgByi5iNyfOlSTOby50mcsUBdrHtIb1PQUtyGv52dZt4s3p+0PPiA7AloZAg",
      "timestamp": "1677586643964"
    }
  ],
  "checkpoint": {
    "origin": "2023-presidential-election2",
    "size": "2",
    "hash": "jyVZBF0Zbkx6gdgvzJKGAlUAGfbLw8A1/TRQSv5+nDI="
  }
}
```
</details>

You can list the entries of a board with:


<details>
  <summary>Click to expand</summary>

```bash
grpcurl \
    -emit-defaults \
    -plaintext \
    -import-path ./proto \
    -proto bulletin_board.proto \
    -d '{"board_uuid": "e04bd3c6-1beb-4ef6-9d65-c411ba7c6d08", "start_sequence_id": 0}' \
    '127.0.0.1:3000' \
    bulletin_board.BulletinBoard/ListEntries
```
</details>

Resulting in something like:

<details>
  <summary>Click to expand</summary>

```json
{
  "boardLastSequenceId": "1",
  "boardEntries": [
    {
      "sequenceId": "0",
      "board": {
        "uuid": "e04bd3c6-1beb-4ef6-9d65-c411ba7c6d08",
        "name": "2023-presidential-election2",
        "description": "",
        "isArchived": false,
        "publicKey": "2023-presidential-election2+dd65551b+AYxh4fg7bo3DQivAcmKh5ypJ/Si0A1b2AozqWWacINDn",
        "metadata": {
          
        },
        "permissions": {
          "users": [
            {
              "name": "admin",
              "publicKey": "4wGsjyIMJHUsGjqnoIKEGD6m40bY1zhc7Jh3ob277+s",
              "metadata": {
                
              }
            }
          ],
          "roles": [
            {
              "name": "admins",
              "permissions": [
                "AddEntries"
              ],
              "metadata": {
                
              }
            }
          ],
          "userRoles": [
            {
              "userName": "admin",
              "roleNames": [
                "admins"
              ]
            }
          ]
        }
      },
      "metadata": {
        
      },
      "signerPublicKey": "4wGsjyIMJHUsGjqnoIKEGD6m40bY1zhc7Jh3ob277+s",
      "signature": "ts2K23wBzn6lprIuwtdcJ3C4qK4fFbxKa3+1CNRRCxF74aZiOjTldOjgmIUrE4ce/6mwT0cfLQL+Py/GWsKKDw",
      "timestamp": "1677586474371"
    },
    {
      "sequenceId": "1",
      "entryData": {
        "data": "SGVsbG8gQm9hcmQh"
      },
      "metadata": {
        
      },
      "signerPublicKey": "4wGsjyIMJHUsGjqnoIKEGD6m40bY1zhc7Jh3ob277+s",
      "signature": "pHUrPXrvPGOmqv8Apc3exoNqnFbgByi5iNyfOlSTOby50mcsUBdrHtIb1PQUtyGv52dZt4s3p+0PPiA7AloZAg",
      "timestamp": "1677586643964"
    }
  ]
}
```

</details>

You can archive a board with:

<details>
  <summary>Click to expand</summary>

```bash
grpcurl \
    -emit-defaults \
    -plaintext \
    -import-path ./proto \
    -proto bulletin_board.proto \
    -d '{"board_uuid": "e04bd3c6-1beb-4ef6-9d65-c411ba7c6d08", "archive": true}' \
    '127.0.0.1:3000' \
    bulletin_board.BulletinBoard/ArchiveBoard
```

</details>

Resulting in something like:

<details>
  <summary>Click to expand</summary>

```json
{
  "board": {
    "uuid": "e04bd3c6-1beb-4ef6-9d65-c411ba7c6d08",
    "name": "2023-presidential-election",
    "description": "",
    "isArchived": true,
    "publicKey": "2023-presidential-election+6f8c9f81+Abwac2R0XtmiAzmDA7/BO5kVzTCA/fhhyK6Uu/j3CPr6",
    "metadata": {
        
    },
    "permissions": {
        "users": [
        {
            "name": "admin",
            "publicKey": "gS2o6mE/9PmDYXHqcqKfyfHSVsoKuEip+olBk3YiQCM",
            "metadata": {
            
            }
        }
        ],
        "roles": [
        {
            "name": "admins",
            "permissions": [
            "AddEntries"
            ],
            "metadata": {
            
            }
        }
        ],
        "userRoles": [
        {
            "userName": "admin",
            "roleNames": [
            "admins"
            ]
        }
        ]
    }
  }
}
```

</details>

## Significant dependencies

* gRPC is implemented using [tonic].
* The log uses components from [trillian] and [trillian-examples/serverless],
  and as such is implemented in Go as a static library in the `trillian-board/`
  directory.
* `trillian-board` exports a C FFI Go interface using [Cgo].
* `backend_trillian.rs` creates a Rust wrapper around the `trillian-board` C FFI
  interface.

## Continuous Integration

There are multiple checks executed through the usage of Github Actions to verify
the health of the code when pushed:
1. **Compiler warning/errors**: checked using `cargo check` and 
`cargo check ---tests`. Use `cargo fix` and `cargo fix --tests` to fix the 
issues that appear.
2. **Unit tests**: check that all unit tests pass using `cargo test`.
3. **Code style**: check that the code style follows standard Rust format, using
`cargo fmt -- --check`. Fix it using `cargo fmt`.
4. **Code linting**: Lint that checks for common Rust mistakes using 
`cargo clippy`. You can try to fix automatically most of those mistakes using
`cargo clippy --fix -Z unstable-options`.
5. **Code coverage**: Detects code coverage with [cargo-tarpaulin] and pushes
the information (in main branch) to [codecov].
6. **License compliance**: Check using [REUSE] for license compliance within
the project, verifying that every file is REUSE-compliant and thus has a 
copyright notice header. Try fixing it with `reuse lint`.
7. **Dependencies scan**: Audit dependencies for security vulnerabilities in the
[RustSec Advisory Database], unmaintained dependencies, incompatible licenses
and banned packages using [ORT]. We also have configured [dependabot] to notify
and create PRs on version updates.
1. **Benchmark performance**: Check benchmark performance and alert on
regressions using `cargo bench` and [github-action-benchmark].
1. **CLA compliance**: Check that all committers have signed the 
[Contributor License Agreement] using [CLA Assistant bot].

## Development environment

bulletin-board uses [Github dev containers] to facilitate development. To start
developing bulletin-board, clone the github repo locally, and open the folder in
Visual Studio Code in a container. This will configure the same environment that
bulletin-board developers use, including installing required packages and VS
Code plugins.

We've tested this dev container for Linux x86_64 and Mac Os arch64
architectures. Unfortunately at the moment it doesn't work with Github
Codespaces as nix doesn't work on Github Codespaces yet. Also the current dev
container configuration for bulletin-board doesn't allow commiting to the git
repo from the dev container, you should use git on a local terminal.

## Nix reproducible builds

bulletin-board uses the [Nix Package Manager] as its package
builder. To build bulletin-board, **first [install Nix]** correctly
in your system. If you're running the project on a dev container,
you shouldn't need to install it.

After you have installed Nix, enter the development environment with:

```bash
nix develop
```

## Updating Cargo.toml

Use the following [cargo-edit] command to upgrade dependencies to latest
available version. This can be done within the `nix develop` environment:

```bash
cargo update
```

## building

This project uses [nix] to create reproducible builds. In order to build the
project as a library for the host system, run:

```nix build```

If you don't want to use nix, you can build the project with:

```cargo build```

## unit tests

```cargo test```

## Troubleshooting

### Docker container fails to build with errors related to Nix

It might be that nix is not found, or similar. This might be because it
has been a long time since you rebuilt the container from zero, with no
cache, and some cached dependencies are no longer available.

Please try to rebuild from scratch the docker container:
- Stop the container
- Remove the container
- Remove the volume
- Perform a `docker system prune --volumes`
- Rebuild the container

This will remove all your containers and images, so please beaware and
do not remove what you shouldn't - you can lose valuable data.

After this, you can regenerate the `flake.lock` file to be updated
and fetch the latest available versions of the dependencies with 
`nix flake update` and then check the build still works with
`nix build`. Finally, commit and push the changes.


[cargo-edit]: https://crates.io/crates/cargo-edit
[codecov]: https://codecov.io/
[REUSE]: https://reuse.software/
[cargo-tarpaulin]: https://github.com/xd009642/tarpaulin
[github-action-benchmark]: https://github.com/benchmark-action/github-action-benchmark
[Contributor License Agreement]: https://cla-assistant.io/sequentech/bulletin-board?pullRequest=27
[CLA Assistant bot]: https://github.com/cla-assistant/cla-assistant
[dependabot]: https://docs.github.com/en/code-security/dependabot/dependabot-version-updates/configuring-dependabot-version-updates
[RustSec Advisory Database]: https://github.com/RustSec/advisory-db/
[Nix Package Manager]: https://nixos.org/
[install Nix]: https://nixos.org/
[Github dev containers]: https://docs.github.com/en/codespaces/setting-up-your-project-for-codespaces/introduction-to-dev-containers
[nix]: https://nixos.org/
[trillian]: https://github.com/google/trillian
[trillian-examples/serverless]: https://github.com/google/trillian-examples/tree/master/serverless
[Cgo]: https://pkg.go.dev/cmd/cgo
[tonic]: https://crates.io/crates/tonic
[ORT]: https://github.com/oss-review-toolkit/ort

[discord-badge]: https://img.shields.io/discord/1006401206782001273?style=plastic
[discord-link]: https://discord.gg/WfvSTmcdY8

[build-badge]: https://github.com/sequentech/bulletin-board/workflows/CI/badge.svg?branch=main&event=push
[build-link]: https://github.com/sequentech/bulletin-board/actions?query=workflow%3ACI

[codecov-badge]: https://codecov.io/gh/sequentech/bulletin-board/branch/main/graph/badge.svg?token=W5QNYDEJCX
[codecov-link]: https://codecov.io/gh/sequentech/bulletin-board

[dependencies-badge]: https://deps.rs/repo/github/sequentech/bulletin-board/status.svg
[dependencies-link]: https://deps.rs/repo/github/sequentech/bulletin-board

[license-badge]: https://img.shields.io/github/license/sequentech/bulletin-board?label=license
[license-link]: https://github.com/sequentech/bulletin-board/blob/main/LICENSE

[reuse-badge]: https://api.reuse.software/badge/github.com/sequentech/bulletin-board
[reuse-link]: https://api.reuse.software/info/github.com/sequentech/bulletin-board

[Documentation]: https://sequent-bulletin-board-docs.netlify.app/
