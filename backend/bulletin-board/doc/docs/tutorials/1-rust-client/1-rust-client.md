---
title: "Tutorial 1: Create a Rust client"
sidebar_position: 1
---

In [Tutorial 0: Get Started] we managed to deploy a BBS, create a board,
add an entry and check that it was indeed added.

We did all that manually, using the BBS gRPC API through the handy `grpcurl`
command. In this tutorial we'll do something similar, but instead we'll do it
programatically in Rust.

We'll assume you have the BBS up and running in a VS Code that has clonned
locally the `bulletin-board` repository. Please refer to the previous tutorial's
steps 1 to 4 for indication on how to do that in a development environment.

We're also assuming basic Rust Language knowledge. We won't be doing anything
very advanced but still it's recommended to be familiarized with how it works.
The [Rust by Example] guide is a good way to start.

If you want to go directly to access the full final code of this tutorial, you
can find it in the repository in the [examples/client.rs file].

## Step 1: Create a new crate

Our rust project will be a new crate, that will use and link to the bulletin 
board client library. In the `/workspace` folder, we create the crate with:

```bash
cargo new bbs_client --bin
```

This create the following file structure in `/workspace/bbs_client/`:

```bash
code:server/workspace$ tree bbs_client/
bbs_client/
|-- Cargo.toml
`-- src
    `-- main.rs

1 directory, 2 files
```

Right now it's just a simple but working Hello World kind of project. Let's test
it works:

```bash
code@1b6688be970f:/workspace/bbs_client$ cargo run
   Compiling bbs_client v0.1.0 (/workspace/bbs_client)
    Finished dev [unoptimized + debuginfo] target(s) in 5.79s
     Running `target/debug/bbs_client`
Hello, world!
```

It does! It just prints `Hello world!`.

## Step 2: Add dependencies

We are going to first modify `Cargo.toml` to add the dependencies we'll use in
this tutorial:

```toml
[package]
name = "bbs_client"
version = "0.1.0"
edition = "2021"

[dependencies]
# anyhow allows us to manage errors easily
anyhow = "1.0"
# format used for configuration
toml = "0.7"
# borsh is used for serialization and deserialization
borsh = "0.9"
# bulletin-board library
bulletin-board = { git = "https://github.com/sequentech/bulletin-board", branch = "main" }
# used for cryptography (public and private keys to verify authentication and
# authorization).
strand = { git = "https://github.com/sequentech/strand", features= ["rayon"], tag = "v0.2.0" }
# clap is used to process executable arguments
clap = { version = "4.0", features = ["derive"] }
# tracing is used for logging
tracing = "0.1"
# uuid v4 feature lets you generate random UUIDs
uuid = { version = "1.2", features = [ "v4" ] }
# for convenience async processing
tokio = { version = "1.24", features = ["macros"] }
```

If we re-run `cargo run`, it should download all the dependencies, rebuild and
run again our `bbs_client` binary.

## Step 3: Generate boilerplate code structure

We're going to modify the source code of the `bbs_client` crate, which lives
in the `src/main.rs` file. Right now it's pretty bare:

```rust
fn main() {
    println!("Hello, world!");
}
```

Let's modify and morph it into the following code:

```rust
use anyhow::{anyhow, Result};
use bulletin_board::client::{Client, FileCache};
use bulletin_board::util::{init_log, KeyPairConfig};
use clap::Parser;
use std::fs;
use std::path::PathBuf;
use strand::signature::StrandSignatureSk as SecretKey;
use tracing::{info, instrument};

/// Struct used to receive the command line arguments of this example
#[derive(Parser)]
struct Cli {
    #[arg(long, value_name = "DIR_PATH")]
    cache_dir: PathBuf,

    #[arg(long)]
    server_url: String,

    #[arg(long, value_name = "FILE_PATH")]
    key_pair_path: PathBuf,
}

#[tokio::main]
#[instrument]
async fn main() -> Result<()> {
    init_log().map_err(|error| anyhow!(error))?;
    let args = Cli::parse();
    let keys_pair_str: String = fs::read_to_string(args.key_pair_path)?;
    let key_pair: KeyPairConfig = toml::from_str(&keys_pair_str)?;
    let secret_key: SecretKey = key_pair.secret_key.try_into()?;

    let cache = FileCache::new(&args.cache_dir)?;
    let mut client = Client::new(args.server_url, cache).await?;
    info!("client created");
    Ok(())
}
```

We have done several things:
1. Import all the symbols we'll use at the begining.
2. Create the `Cli` struct, that defines the information this command requires
   as input parameters to be launched. For example, the BBS client caches in a
   local folder entries and other data to be more efficient and we need to
   specify a folder to use for caching. It also requires the URL of the BBS
   server and finally the path to the file holding our public and private key -
   we can use the key created in [Tutorial 0: Get Started] in the `$PUBLIC_KEY`
   path.
3. Since we are using `await` functions in `main()`, we have converted the
   `main()` function in `async`. We also wrap it with the `#[tokio::main]` macro
   to be able to use async for the main function.
4. We use the utility function `init_log()` to configure logging using the
   `tracing` crate, and also use the `info!()` macro to create an informative
   trace.
5. Within the `main()` function we mainly parse the `Cli` arguments, then parse
   and load the public and private key pair, initialize the `FileCache` and
   from that and the server url instantiate the BBS `Client`. We don't use it
   yet but it's ready to go.

We can run our simple client with:

```bash
# Change the storage dir to whatever was printed in the server terminal
export STORAGE_DIR=/tmp/tmp.8J4EbZEejY

# create a new cache dir
export CACHE_DIR=$(mktemp -d)

# run the example client
TRACING_LEVEL=info \
cargo run -- \
   --cache-dir "${CACHE_DIR}" \
   --server-url "https://127.0.0.1:3000" \
   --key-pair-path "${STORAGE_DIR}/client-keys.toml"
```

Which should show an output similar to:

```
code@1b6688be970f:/workspace/bbs_client$ TRACING_LEVEL=info cargo run --    --cache-dir "${CACHE_DIR}"    --server-url "https://127.0.0.1:3000"    --key-pair-path "${STORAGE_DIR}/client-keys.toml"
[compiler output]
     Running `target/debug/bbs_client --cache-dir /tmp/tmp.id3Cwdqbqg --server-url 'https://127.0.0.1:3000' --key-pair-path /tmp/tmp.8J4EbZEejY/client-keys.toml`
 INFO tracing_level_str="info"
┐new cache_dir="/tmp/tmp.id3Cwdqbqg"
┘
┐new server_url="https://127.0.0.1:3000"
┘
 INFO client created
```

You'll see a couple of warnings during compilation because we created a few
variables we are not yet using. Our client doesn't do much, really: it just
setups everything, connects to the gRPC API and then prints "client created" and
closes shop. 

But you can see that our client actually connected to the server in the BBS debug
log output in the other terminal:

```bash
DEBUG send, frame=Settings { flags: (0x0), initial_window_size: 1048576, max_frame_size: 16384, max_header_list_size: 16777216 }
┐Connection peer=Server
├──0ms DEBUG send, frame=WindowUpdate { stream_id: StreamId(0), size_increment: 983041 }
├──1ms DEBUG received, frame=Settings { flags: (0x0), enable_push: 0, initial_window_size: 2097152, max_frame_size: 16384 }
├──1ms DEBUG send, frame=Settings { flags: (0x1: ACK) }
├──1ms DEBUG received, frame=Settings { flags: (0x1: ACK) }
├──2ms DEBUG received settings ACK; applying Settings { flags: (0x0), initial_window_size: 1048576, max_frame_size: 16384, max_header_list_size: 16777216 }
├──2ms DEBUG received, frame=WindowUpdate { stream_id: StreamId(0), size_increment: 5177345 }
├──3ms DEBUG received, frame=GoAway { error_code: NO_ERROR, last_stream_id: StreamId(0) }
DEBUG connection error: connection error: not connected
┘
DEBUG send, frame=Settings { flags: (0x0), initial_window_size: 1048576, max_frame_size: 16384, max_header_list_size: 16777216 }
┐Connection peer=Server
├──18ms DEBUG received, frame=Settings { flags: (0x0), enable_push: 0, initial_window_size: 2097152, max_frame_size: 16384 }
├──18ms DEBUG send, frame=Settings { flags: (0x1: ACK) }
├──18ms DEBUG received, frame=WindowUpdate { stream_id: StreamId(0), size_increment: 5177345 }
├──18ms DEBUG received, frame=GoAway { error_code: NO_ERROR, last_stream_id: StreamId(0) }
DEBUG connection error: connection error: broken pipe
```

Typically this doesn't show up in the BBS log output, but it does because we enabled DEBUG output. It just reflects a client connected and then closed it.

## Step 4: Fully use the Bulletin Board API

Let's modify the `main.rs` to become:

```rust
use anyhow::{anyhow, Result};
use borsh::{BorshDeserialize, BorshSerialize};
use bulletin_board::client::{Client, FileCache};
use bulletin_board::signature::Signable;
use bulletin_board::util::{init_log, KeyPairConfig};
use bulletin_board::{
    board_entry::Kind, AddEntriesRequest, BoardEntry, BoardEntryData,
    CreateBoardRequest, ListBoardItem, ListBoardsRequest, ListEntriesRequest,
    NewDataEntry, Permissions, Role, User, UserRole,
};
use clap::Parser;
use std::fs;
use std::path::PathBuf;
use strand::signature::StrandSignatureSk as SecretKey;
use tracing::{error, info, instrument};
use uuid::Uuid;

/// Struct used to receive the command line arguments of this example
#[derive(Parser)]
struct Cli {
    #[arg(long, value_name = "DIR_PATH")]
    cache_dir: PathBuf,

    #[arg(long)]
    server_url: String,

    #[arg(long, value_name = "FILE_PATH")]
    key_pair_path: PathBuf,
}

/// Simple example demonstrating how to use the bulletin board client, using a
/// local file-based cache. We list boards, create one if it doesn't exist, add
/// an entry and retrieve the added entry
#[tokio::main]
#[instrument]
async fn main() -> Result<()> {
    init_log().map_err(|error| anyhow!(error))?;
    let args = Cli::parse();
    let keys_pair_str: String = fs::read_to_string(args.key_pair_path)?;
    let key_pair: KeyPairConfig = toml::from_str(&keys_pair_str)?;
    let secret_key: SecretKey = key_pair.secret_key.try_into()?;

    let cache = FileCache::new(&args.cache_dir)?;
    // highlight-next-line
    let mut client = Client::new(args.server_url, cache).await?;
    info!("client created");

    // highlight-start
    // list boards, and find if ours exists
    let board_name = "example-board".to_string();
    let request = ListBoardsRequest {
        board_name: Some(board_name.clone()),
        ..Default::default()
    };
    info!("Retrieving board with board_name='{board_name}'..");
    let response = client.list_boards(request).await?;
    let boards: &Vec<ListBoardItem> = &response.get_ref().boards;
    info!("Received {} board(s)", boards.len());

    // obtain the board uuid, creating the board if need be
    let board_uuid: String = match boards.len() {
        0 => {
            // create the board and return board uuid
            let board_uuid = Uuid::new_v4().to_string();
            let request = CreateBoardRequest {
                board_uuid: board_uuid.clone(),
                board_name: board_name.clone(),
                permissions: Some(Permissions {
                    users: vec![User {
                        name: "admin".into(),
                        public_key: key_pair.public_key.clone(),
                        ..Default::default()
                    }],
                    roles: vec![Role {
                        name: "admins".into(),
                        permissions: vec!["AddEntries".into()],
                        ..Default::default()
                    }],
                    user_roles: vec![UserRole {
                        user_name: "admin".into(),
                        role_names: vec!["admins".into()],
                    }],
                    ..Default::default()
                }),
                ..Default::default()
            }
            .sign(&secret_key)
            .unwrap();
            info!("Board not found, creating it..");
            let _response = client.create_board(request).await?;
            info!("Board created with uuid={board_uuid}");
            board_uuid
        }
        _ => {
            // get the board uuid from response
            let ListBoardItem { board, .. } =
                boards.get(0).ok_or(anyhow!("BoardRetrievalError"))?;

            let board_uuid = board.clone().ok_or(anyhow!("EmptyBoard"))?.uuid;
            info!("Board with name='{board_name}' has uuid='{board_uuid}'");
            board_uuid
        }
    };

    // let's list entries in the board
    let request = ListEntriesRequest {
        board_uuid: board_uuid.clone(),
        start_sequence_id: 0,
    };
    info!("Listing board entries..");
    let response = client.list_entries(request).await?;
    info!(
        "List board entries response contains {} entries",
        response.get_ref().board_entries.len()
    );

    // let's add a new entry
    let new_entry_string = String::from("Hello World!");
    let request = AddEntriesRequest {
        board_uuid: board_uuid.clone(),
        entries: vec![NewDataEntry {
            data: new_entry_string.try_to_vec()?,
            ..Default::default()
        }
        .sign(&secret_key)
        .unwrap()],
    };
    info!("Adding new entry with data '{new_entry_string}' to the board..");
    let response = client
        .add_entries(request, /* update_cache = */ true)
        .await?;

    // print the entry sequence id
    let entry_sequence_id = response.get_ref().entries[0].sequence_id;
    info!("New entry added with entry_sequence_id={entry_sequence_id:?}");

    // if we try to get the new entry, it will be cached - you won't see a new
    // call to list_entries in the bulletin board service
    let request = ListEntriesRequest {
        board_uuid: board_uuid.clone(),
        start_sequence_id: entry_sequence_id,
    };
    info!("Retrieving the added entry..");
    let response = client.list_entries(request).await?;
    info!(
        "List entries response contains {} entries",
        response.get_ref().board_entries.len()
    );

    // deserialize the entry we just created and retrieved
    let entry: &BoardEntry = response
        .get_ref()
        .board_entries
        .get(0)
        .ok_or(anyhow!("MisingBoardEntry"))?;
    if let Some(Kind::EntryData(BoardEntryData {
        data: Some(entry_data),
    })) = &entry.kind
    {
        let hello_world = String::try_from_slice(entry_data)?;
        info!("The added entry data string is '{hello_world}'");
    } else {
        error!("Unexpected/invalid entry: {entry:?}");
    }
    // highlight-end

    Ok(())
}
```

Apart from changing the imports, we have converted our client to be a fully
featured example. We are using all the APIs:
- We list available boards filtering by the `"example-board"` name, to see if
the board already exists. This allows for the client not to fail if executed
twice, since it won't try to create the board if it does exist.
- If the board does not exist, we create our example board. If it does exist, we
obtain the board uuid and use it.
- We list the entries in the board.
- We add a new entry with content `"Hello World!"`.
- We retrieve new entries (starting from the last cached entry), checking that
these now include the new entry.

If we execute again our `main.rs` binary, we should get output similar to the
following:

```bash
code@1b6688be970f:/workspace/bbs_client$ TRACING_LEVEL=info cargo run --    --cache-dir "${CACHE_DIR}"    --server-url "https://127.0.0.1:3000"    --key-pair-path "${STORAGE_DIR}/client-keys.toml"
   Compiling bbs_client v0.1.0 (/workspace/bbs_client)
    Finished dev [unoptimized + debuginfo] target(s) in 47.36s
     Running `target/debug/bbs_client --cache-dir /tmp/tmp.id3Cwdqbqg --server-url 'https://127.0.0.1:3000' --key-pair-path /tmp/tmp.8J4EbZEejY/client-keys.toml`
 INFO tracing_level_str="info"
┐new cache_dir="/tmp/tmp.id3Cwdqbqg"
┘
┐new server_url="https://127.0.0.1:3000"
┘
 INFO client created
 INFO Retrieving board with board_name='example-board'..
 INFO Received 0 board(s)
 INFO Board not found, creating it..
 INFO Board created with uuid=92a6af3b-471e-4d16-8d8a-02ec81029410
 INFO Listing board entries..
 INFO List board entries response contains 1 entries
 INFO Adding new entry with data 'Hello World!' to the board..
 INFO New entry added with entry_sequence_id=1
 INFO Retrieving the added entry..
 INFO List entries response contains 1 entries
 INFO The added entry data string is 'Hello World!'
 ```

 ## Step 5: Adding more entries, playing with the cache

 If we execute it more times, it will still work, and instead of creating a new
 board it will keep adding entries to the same board uuid:

 ```bash
code@1b6688be970f:/workspace/bbs_client$ TRACING_LEVEL=info cargo run --    --cache-dir "${CACHE_DIR}"    --server-url "https://127.0.0.1:3000"    --key-pair-path "${STORAGE_DIR}/client-keys.toml"
    Finished dev [unoptimized + debuginfo] target(s) in 4.27s
     Running `target/debug/bbs_client --cache-dir /tmp/tmp.id3Cwdqbqg --server-url 'https://127.0.0.1:3000' --key-pair-path /tmp/tmp.8J4EbZEejY/client-keys.toml`
 INFO tracing_level_str="info"
┐new cache_dir="/tmp/tmp.id3Cwdqbqg"
┘
┐new server_url="https://127.0.0.1:3000"
┘
 INFO client created
 INFO Retrieving board with board_name='example-board'..
 INFO Received 1 board(s)
 INFO Board with name='example-board' has uuid='92a6af3b-471e-4d16-8d8a-02ec81029410'
 INFO Listing board entries..
 INFO List board entries response contains 2 entries
 INFO Adding new entry with data 'Hello World!' to the board..
 INFO New entry added with entry_sequence_id=2
 INFO Retrieving the added entry..
 INFO List entries response contains 1 entries
 INFO The added entry data string is 'Hello World!'
 ```

 We can also see that the cache is working and the cache structure just by
 executing `tree $CACHE_DIR`:

```bash
code@1b6688be970f:/workspace/bbs_client$ tree $CACHE_DIR
/tmp/tmp.id3Cwdqbqg
|-- board_config__92a6af3b-471e-4d16-8d8a-02ec81029410.json
|-- board_entry__92a6af3b-471e-4d16-8d8a-02ec81029410_0
|-- board_entry__92a6af3b-471e-4d16-8d8a-02ec81029410_1
|-- board_entry__92a6af3b-471e-4d16-8d8a-02ec81029410_2
`-- board_ids.json

0 directories, 5 files
```

The cache directory is not required to be filled to work. It just needs to be
an empty directory. We can erase it, and our client will work just fine:

```bash
code@1b6688be970f:/workspace/bbs_client$ rm $CACHE_DIR/*
code@1b6688be970f:/workspace/bbs_client$ TRACING_LEVEL=info cargo run --    --cache-dir "${CACHE_DIR}"    --server-url "https://127.0.0.1:3000"    --key-pair-path "${STORAGE_DIR}/client-keys.toml"
    Finished dev [unoptimized + debuginfo] target(s) in 4.27s
     Running `target/debug/bbs_client --cache-dir /tmp/tmp.id3Cwdqbqg --server-url 'https://127.0.0.1:3000' --key-pair-path /tmp/tmp.8J4EbZEejY/client-keys.toml`
 INFO tracing_level_str="info"
┐new cache_dir="/tmp/tmp.id3Cwdqbqg"
┘
┐new server_url="https://127.0.0.1:3000"
┘

 INFO client created
 INFO Retrieving board with board_name='example-board'..
 INFO Received 1 board(s)
 INFO Board with name='example-board' has uuid='92a6af3b-471e-4d16-8d8a-02ec81029410'
 INFO Listing board entries..
 INFO List board entries response contains 3 entries
 INFO Adding new entry with data 'Hello World!' to the board..
 INFO New entry added with entry_sequence_id=3
 INFO Retrieving the added entry..
 INFO List entries response contains 1 entries
 INFO The added entry data string is 'Hello World!'
```

It works fine, we keep adding entries and the cache is repopulated:

```bash
code@1b6688be970f:/workspace/bbs_client$ tree ${CACHE_DIR}
/tmp/tmp.id3Cwdqbqg
|-- board_config__92a6af3b-471e-4d16-8d8a-02ec81029410.json
|-- board_entry__92a6af3b-471e-4d16-8d8a-02ec81029410_0
|-- board_entry__92a6af3b-471e-4d16-8d8a-02ec81029410_1
|-- board_entry__92a6af3b-471e-4d16-8d8a-02ec81029410_2
|-- board_entry__92a6af3b-471e-4d16-8d8a-02ec81029410_3
`-- board_ids.json

0 directories, 6 files
```

Of course, if entries are large, it's better to keep the local cache so that we
don't send more load than necessary to the BBS and to speed up the client
operations. The BBS is in the end a linear log of entries, and having to 
retrieve a potentially large number of potentially large entries each time isn
not a good idea. 

## Conclusion

We have implemented a simple BBS client in Rust. It supports all basic
operations, including log creation, looking up a board by id, reading log
entries and writing new log entries. We have made use of the cache to make this
efficient too.

## Next steps

You can learn more about what you can do in a BBS client by taking a look at the:
- <a href="/rust-api/" target="_blank">Rust API</a>
- [gRPC API](./reference/protobuf.md)

For example, when we created the Bulletin Board we assigned a permission set.
You can change that (and other parameters of the board) with
`ModifyBoardRequest` available in the gRPC API.

You can also learn more about the file format in which Boards are stored in the
BBS and the file format used for the cache directory in the Data Schemas section
of the [Reference guides](../../reference/).

[Tutorial 0: Get Started]: ../get-started
[Rust by Example]: https://doc.rust-lang.org/rust-by-example/
[examples/client.rs file]: https://github.com/sequentech/bulletin-board/blob/main/examples/client.rs
