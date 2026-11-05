---
id: ballot_image_signatures
title: Ballot Image Signatures
sidebar_position: 12
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

When multi-contest ballot images are generated for an election event that has
an ACM key, two things are signed with that key:

- Every contest page has signed data and a base64 signature. The default
  template prints both in the page QR code as `<signed data>:<signature>`.
- The `ballots_files.csv` list of generated PDF files comes with
  `ballots_files.csv.sign`, the base64 signature of the exact bytes of the CSV.

Both signatures are ECDSA with SHA-256 and can be checked with the ACM public
key of the election event.

## Page signature policy

The election event annotation `ballot-images:signature-policy` selects what the
page signatures cover:

| Value | Signed data on each page |
| --- | --- |
| `IDENTIFIERS_ONLY` | `event:precinct:serial:election:page`. Applied when the annotation is absent. |
| `IDENTIFIERS_AND_CHOICES` | A versioned payload with the same identifiers, the contest id and a hash of the selected candidates. |

Set it in the `annotations` of the election event, for example
`"ballot-images:signature-policy": "IDENTIFIERS_AND_CHOICES"`. An unrecognised
value stops ballot image generation with an error. Before choosing
`IDENTIFIERS_AND_CHOICES`, confirm that every tool that reads the page QR codes
accepts the `v2` payload described below.

The identifiers are:

- **event**: the `miru:election-event-id` annotation of the election event.
- **precinct**: the `miru:precinct-code` annotation of the election.
- **serial**: the serial number of the ballot.
- **election**: the `miru:election-id` annotation of the election.
- **page**: the page number inside the generated file.

### The `v2` payload

With `IDENTIFIERS_AND_CHOICES`, the signed data is `v2` followed by seven
fields, each written as `:<length>:<value>`, where `<length>` is the length of
the value in bytes (UTF-8):

1. event
2. precinct
3. serial
4. election
5. page
6. contest id
7. selections hash: the lowercase hexadecimal SHA-256 of the ids of the
   candidates selected in the contest, sorted by byte value, each written as
   `<length>:<id>` and joined with `:`. A contest with no selection hashes the
   empty string.

For example, a page of contest `contest` with candidates `a` and `c` selected
signs:

```text
v2:5:event:4:prec:9:000000001:4:elec:1:3:7:contest:64:b2939ab7982fed32555ff3b42e4361401f8a511e9f52c81f58e0fbeea7573fdb
```

where the last field is the SHA-256 of `1:a:1:c`.

## Checking a set of ballot images

1. Check `ballots_files.csv` against `ballots_files.csv.sign` with the ACM
   public key, for example
   `java -jar ecies-tool.jar verify acm-public-key.pem ballots_files.csv "$(cat ballots_files.csv.sign)"`.
2. Check that each PDF file is listed in the CSV, that each listed file is
   present, and that the hexadecimal SHA-256 of each PDF equals the last
   `_`-separated part of its file name, before `.pdf`.
3. For each page, split the QR text at its last `:` into the signed data and
   the signature, and check the signature with the ACM public key. With
   `IDENTIFIERS_AND_CHOICES`, also recompute the selections hash from the
   candidates marked on the page and compare it with the last field.
