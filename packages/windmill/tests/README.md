<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Windmill boundary tests

These public integration tests cover encryption-password confidentiality,
canonical import arithmetic and errors, material changes in reviewed tally
sheets, CSV display fidelity, and SQL escaping through PostgreSQL's real parser.
No test sends mail, SMS or election data to an external service.

Use PostgreSQL 16 on loopback port 3322 with the synthetic `test` user/password
and database, matching `.github/workflows/tests.yml`. Each query test uses its
own transaction; the unsafe-string-mode test uses a separate connection option.
It never changes shared database defaults. The coverage profile provides these
public fixture settings; it must run in the isolated worker, not on production.

The existing ignored voter-channel PostgreSQL regression is run explicitly as
separate evidence. The other ignored activity-log test requires a fuller
service fixture and must remain visible in the report. Native default FIPS
coverage does not certify cloud transports, full election services, optional
features or branches. Do not exclude untested workers to reach 95%.

With PostgreSQL running, measure from the repository root:

```bash
python3 scripts/coverage/run.py windmill --baseline --offline
```

Run the existing voter-channel regression explicitly from `packages/`:

```bash
KEYCLOAK_DB__HOST=127.0.0.1 \
HASURA_DB__HOST=127.0.0.1 HASURA_DB__PORT=3322 \
HASURA_DB__USER=test HASURA_DB__PASSWORD=test HASURA_DB__DBNAME=test \
LOW_SQL_LIMIT=1000 DEFAULT_SQL_LIMIT=20 DEFAULT_SQL_BATCH_SIZE=1000 \
cargo test -p windmill --locked --offline --lib -- \
  services::cast_votes::tests::voters_by_channel_defaults_legacy_votes_and_uses_latest_valid_revote \
  --ignored --exact
```

This separate command does not add its counters to a previous coverage report.
