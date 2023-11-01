# Immutable Logging

This document contains an overview of how we use Immudb some details related to 
each of these uses.

## Introduction: Immutable logging with Immudb

We use [Immudb] for our tamper-evident logging needs. Immudb is an "Open source
immutable database" with the following characteristics:
- High performance and easy to integrate
- Supports both Key/Value & SQL
- Cryptographical client-verification, tamper-resistant, and auditable
- Versioned and temporal queries subject to cryptographic verification

## 3 Different immutable logs

There are three different manners in which we use Immudb:

1. **Backend Database Log:** Our Backend API implements a GraphQL interface.
   This API is exposed by Hasura. The business logic is implemented by the
   `harvest` and `windmill` packages. The Backend API stores data in a
   PostgreSQL database. Changes to this database, and sometimes queries (based
   on settings), get logged by [PgAudit]. We record these logs in an Immudb
   Database. For this process, we use a vendored fork of `immudb-log-audit` in
   Go. You can find it in our repository under `vendor/immudb-log-audit/`.

   Database logging happens at the deployment level. There's a one to one
   relation between the deployment's single PostgreSQL database and the 
   Immudb immutable log populated with PgAudit.

2. **Cryptographic Board Log:** Sequent Voting Platform uses a Mixnet for
   preserving ballot secrecy. The mixnet shuffles the ballots in a
   mathematically verifiable manner. All these operations are orchestated and
   logged for transparency using an immutable log. This log is the basis of the
   Cryptographic Board. The Cryptographic Board is implemented using an Immudb
   Database. The Cryptographic Board defines a protocol for its operations. This
   protocol allows the actors involved to know what has happened. It also allows
   actors know if they have any next action to execute in the protocol.

   There's one Cryptographic Board per Election Event. Each Cryptographic Board
   has its own Immudb database.

3. **Election Protocol Log:** Important election operations need to be signed by
   one or more people and correctly registered in the Election Protocol Log. The
   Election Protocol register operations such as each cast vote by a voter or
   the request by one or more adminstrators to perform an election tally. These
   operations require the signature of of the person executing the action. Also,
   the backend registering the action signs on Sequent Voting Platform behalf. 
   Finally, the operation and both signatures are recorded in the Election 
   Protocol Board.

   Each Election Event has its own Election Protocol Board. And Each Election
   Protocol Board Log is implemented as an Immudb Database.

[Immudb]: https://immudb.io/
[PgAudit]: https://www.pgaudit.org/