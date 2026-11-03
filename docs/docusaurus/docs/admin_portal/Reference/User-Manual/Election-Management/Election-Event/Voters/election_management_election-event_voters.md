---
id: election_management_election_event_voters
title: Voters
---



This is a placeholder page for the section: Voters.

Content will be added here soon.


### External system (i.e. Datafix) VoterView connection

The `datafix:voterview_request` annotation of the election event holds the
VoterView endpoint `url`, the `usr` and `psw` credentials and the
`county_mun` code.

The optional `transport_policy` field of that annotation sets which URL
schemes are accepted:

- `https-only` (default): the `url` must use `https`.
- `allow-plaintext`: the `url` may use `https` or `http`. Use it only for test
  endpoints.

For example:

```json
{
  "url": "https://voterview.example.org/service.asmx",
  "usr": "user",
  "psw": "password",
  "county_mun": "county",
  "transport_policy": "https-only"
}
```

A request whose `url` does not match the transport policy is not sent.
VoterView redirects are not followed, and responses larger than 1 MiB are
rejected.
