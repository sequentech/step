<!--
SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Examples

This directory contains a series of examples of how to use the Bulletin Board: 

## Client

`client.rs` is an example of how to interact with the bulletin board, using the
convenient interface `bulletin_board::client::Client` that allows using a local
file-based cache of bulletin board entries.

To run it locally, you need to first launch the server on one terminal, for
example with:

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

This will show something like:

```log
export STORAGE_DIR=/tmp/tmp.rTtR9mCxnM
[compiler output]
     Running `target/debug/bulletin-board-server`
 INFO tracing_level_str="debug"
┐read_config 
├──0ms DEBUG reading config_path, config_path_str="/tmp/tmp.rTtR9mCxnM/service-config.toml"
├──┐new service_config=BulletinBoardServiceConfig { storage_path: "/tmp/tmp.rTtR9mCxnM", server_url: "127.0.0.1:3000", permissions: Permissions { users: [User { name: "admin", public_key: "CUpmqQf1R98HyPm2auhzKMIH/AwFWdoixdwgKCT4bBo", metadata: {} }], roles: [Role { name: "admins", permissions: ["CreateBoard"], metadata: {} }], user_roles: [UserRole { user_name: "admin", role_names: ["admins"] }] } }
├──┘
┘
 INFO Launching the bulletin board server, addr=127.0.0.1:3000
```

Now that we have a bulletin board server running, we can connect to it with the
cached client example, but executing the following in a different terminal:

```bash
# Change the storage dir to whatever was printed in the other terminal before
export STORAGE_DIR=/tmp/tmp.rTtR9mCxnM

# create a new cache dir
export CACHE_DIR=$(mktemp -d)

# run the example client
TRACING_LEVEL=info \
cargo run \
   --features="build-server" \
   --example client -- \
   --cache-dir "${CACHE_DIR}" \
   --server-url "https://127.0.0.1:3000" \
   --key-pair-path "${STORAGE_DIR}/client-keys.toml"
```

Which should show an output similar to:

```
[compiler output]
     Running `target/debug/examples/client --cache-dir /tmp/tmp.KD6EGnpMXe --server-url 'https://127.0.0.1:3000' --key-pair-path /tmp/tmp.rTtR9mCxnM/client-keys.toml`
 INFO tracing_level_str="info"
┐new cache_dir="/tmp/tmp.KD6EGnpMXe"
┘
┐new server_url="https://127.0.0.1:3000"
┘
 INFO client created
 INFO Retrieving board with board_name='example-board'..
 INFO Received 0 board(s)
 INFO Board not found, creating it..
 INFO Board created with uuid=57f5226d-bc5e-474d-b98a-67232ccbe3d3
 INFO Listing board entries..
 INFO List board entries response contains 1 entries
 INFO Adding new entry with data 'Hello World!' to the board..
 INFO New entry added with entry_sequence_id=1
 INFO Retrieving the added entry..
 INFO List entries response contains 1 entries
 INFO The added entry data string is 'Hello World!'
```

You can easily see the structure of the cache with `tree`:

```bash
$ tree $CACHE_DIR
/tmp/tmp.KD6EGnpMXe
|-- board_config__57f5226d-bc5e-474d-b98a-67232ccbe3d3.json
|-- board_config__e308b102-e9ee-486a-a4e0-64ebec8587ad.json
|-- board_entry__57f5226d-bc5e-474d-b98a-67232ccbe3d3_0
|-- board_entry__57f5226d-bc5e-474d-b98a-67232ccbe3d3_1
|-- board_entry__e308b102-e9ee-486a-a4e0-64ebec8587ad_0
`-- board_ids.json

0 directories, 6 files
```

[tonic]: https://crates.io/crates/tonic
