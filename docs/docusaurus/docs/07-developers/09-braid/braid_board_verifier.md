---
id: braid_board_verifier
title: Braid Board Verifier
sidebar_label: Braid Board Verifier
---

# Braid Board Verifier

The `verify` binary, shipped in the braid image as `/usr/bin/verify`, reads every message of a board from the B3 bulletin board service and checks the published election data: the configuration, the message signatures, the public key, and for each tallied batch the ballots, the mixing chain, the decryption and the plaintexts.

## Usage

```bash
verify --server-url http://b3:50051 \
  --board <board name> \
  --expected-cfg-hash <hex encoded configuration hash>
```

| Option | Description | Required |
|--------|-------------|----------|
| `--server-url` | URL of the B3 gRPC service | Yes |
| `--board` | Name of the board to verify | Yes |
| `--expected-cfg-hash` | Hex encoded hash of the configuration the board must use | No |

## Configuration hash

Every message on a board refers to the board's configuration, which lists the protocol manager and trustee public keys and the threshold. The verifier always prints the full hex encoded hash of that configuration.

When `--expected-cfg-hash` is given, the verifier stops with an error unless the board's configuration has exactly that hash. Obtain the expected value independently of the board being verified. Without the option, the verifier logs a warning with the board's configuration hash, which must then be compared by hand with its expected value.

## Exit status

The verifier exits with status `0` only when every check passes. It exits with a non-zero status when any check fails, when the configuration hash does not match `--expected-cfg-hash`, or when the board cannot be read. Scripts can rely on the exit status instead of parsing the report.

When the B3 service splits a large board into several replies, the verifier requests the remaining messages until it has read the whole board.
