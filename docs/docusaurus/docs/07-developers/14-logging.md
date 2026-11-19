---
id: logging
title: Service Logging
sidebar_label: Logging
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Service Logging

harvest, windmill (and its workers and beat), velvet, orare and b4 set up
logging through `sequent_core::util::init_log`, configured with the
`LOG_LEVEL` environment variable.

## `LOG_LEVEL`

`LOG_LEVEL` takes a plain level or a comma-separated list of
[`EnvFilter` directives](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html#directives).

| Value | Effect |
| --- | --- |
| unset or empty | `info` |
| `debug` | Everything at `debug`, with the noisy dependencies below kept at `info` |
| `info,windmill::services::ceremonies=warn` | Everything at `info`, except that module and its submodules at `warn` |
| `windmill=debug` | `windmill` at `debug`, everything else at `info` |
| `debug,rocket=debug` | Everything at `debug`, including `rocket` |

- A directive `target=level` applies to every target (module path) that
  starts with `target`.
- Levels are `off`, `error`, `warn`, `info`, `debug` and `trace`, in any case.
- A value that can't be parsed, including a bare word that isn't a level,
  is ignored: the service starts at `info` and logs a warning naming the
  value.

### Dependencies kept at `info`

When the base level is `debug` or `trace`, these targets stay at `info`:
`rocket`, `hyper`, `h2`, `rustls`, `tokio_postgres`, `reqwest`, `lapin` and
`aws_*`. At `debug`, `rocket::server` logs every request with its headers,
including the `Authorization` bearer token. A directive for one of these
targets in `LOG_LEVEL` overrides the default. Turn one on only for as long as needed,
and remove what it logged afterwards.

### Targets in log lines

Every line shows its target, so the module to filter can be read from the
line itself:

```text
INFO rocket::server::_ Matched: (check_private_key) POST /check-private-key application/json
INFO windmill::services::probe return=true
```

## Muting a module that leaks sensitive data

When a log line is found to contain sensitive data, mute the module that
writes it until a release with the fix is deployed:

1. Find the target in the leaking line. For a span, it's the module path
   before the span name; for an event, it's the word after the level.
2. Set the directive in the environment's values, keeping the current
   base level, for example `LOG_LEVEL: "info,windmill::services::ceremonies::keys_ceremony=warn"`.
   An `info` span is not created at all once its module is at `warn`, so
   its fields are not printed.
3. After the pods restart, check in Loki that no new line contains the data.
4. Remove the data already written, and rotate any secret that was exposed.
5. Remove the directive once the fixed release is deployed.

Muting hides every line of that module at the muted levels, not only the
leaking one, so mute the most specific module possible.
