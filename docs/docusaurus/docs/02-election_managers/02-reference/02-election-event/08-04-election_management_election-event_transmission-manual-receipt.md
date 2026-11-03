---
id: election_management_election_event_transmission_manual_receipt
title: Transmission by Manual Receipt
---

<!--
-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The OVCS generates each Post's and country's ER transmission file and lets the
SBEIs download it, so that it can be carried to the canvassing system by hand.
This is manual receipt. Nothing is sent over the network.

On this page, a Post is an election and a country is an area. The **ER file**
is the election returns in the canvassing system's format, EML 510. The SBEIs
sign the ER file. The **transmission package** is `all_servers.zip`. It has
one folder per destination, and each folder holds `er_<station>.zip`, which
contains:

- `er_<station>.exz`: the ER file, compressed and then encrypted with the
  destination's key.
- `er_<station>.json`: the ER file's SHA-256, the OVCS signature and the SBEI
  signatures.

Sending the package to destinations over the network is not part of manual
receipt. It waits until the receiver and its format are known.

## Configuration

Every value the generator reads comes from `miru:` annotations on the election
event, the Post, the country, the contests and the candidates. The sample
configuration in `scripts/transmission-sample/` holds all of them. Use it to
try the flow on any test event.

### Destinations for manual receipt

A destination in the country's `miru:area-ccs-servers` that has no `address`
receives its package by hand:

```json
{
  "name": "CCS Manila (SBOC)",
  "tag": "ccs-manila",
  "address": "",
  "public_key_pem": "-----BEGIN PUBLIC KEY-----\n…\n-----END PUBLIC KEY-----\n",
  "send_logs": false
}
```

The package is sealed for the destination's public key, downloaded, and never
sent. Destinations that have an address keep the network behaviour. When
every destination of a package receives it by hand, the Admin Portal shows no
Send button, and the send action refuses the package.

### The OVCS identity

The package names the OVCS by its device id, serial number, IP address and MAC
address. Windmill, which generates the package, and Harvest, which seals it
again with each uploaded signature, read them from these variables. Both
refuse to seal a package while any of them is missing or malformed:

| Variable | Value |
| --- | --- |
| `MIRU_DEVICE_ID` | The OVCS's device id |
| `MIRU_SERIAL_NUMBER` | Its serial number |
| `MIRU_IP_ADDRESS` | An IPv4 or IPv6 address |
| `MIRU_MAC_ADDRESS` | Six hexadecimal pairs, separated by `:` or `-` |

The development environment sets example values in
`.devcontainer/.env.development`. A deployment sets its own.

### Loading the sample configuration

`scripts/transmission-sample/` contains:

- `transmission-sample-config.json`: the annotations for Madrid PE (the Post)
  and Spain (the country). The values are fictitious. The country has three
  SBEIs, needs two signatures, and has one destination for manual receipt,
  "CCS Manila (SBOC)".
- `kit/`: synthetic keys for tests only.
  - The event's root and intermediate CAs.
  - The SBEIs' certificates, as `.p12` files with password `sample`.
  - The destination's key pair.
  - Another destination's private key, used to check that a package does not
    open with someone else's key.
  - `generate_kit.sh` makes a new kit.
- `load_sample_config.py`: writes the annotations on an event.

Contests and candidates are matched in the order of their English names. The
tally reads their annotations from the published ballot styles, so load the
configuration before publishing:

```bash
scripts/transmission-sample/load_sample_config.py \
    --hasura-url http://127.0.0.1:8080/v1/graphql --admin-secret admin \
    --election-event-id <event> --election-id <post> --area-id <country> \
    --sbei <username of the first signer> --sbei <username of the second signer>
```

The `--sbei` options map the configuration's SBEIs, in order, to the accounts
that sign. `--scope ballot` writes only the contests and candidates.
`--scope package` writes only the event, the Post and the country.

Finally, switch on `ACTIVATE_MIRU_EXPORT` in the Admin Portal's global
settings.

## Generating and downloading the package

1. Tally the Post. Then, under **Tally › results › Export**, choose **Generate
   Transmission Package for Area '&lt;country&gt;'**.
2. The Transmission page opens.
   - The ER file's SHA-256 is under the title, in groups of eight characters,
     so it can be read out and compared with what is received.
   - **Destination Servers** shows **Manual receipt**. Each destination's row
     says "Download the package and hand it over" and gives the SHA-256 of the
     key the package is sealed for.
3. The SBEIs sign. Each signature seals the package again with every signature
   so far. The **Signatures** chip stays orange until the signatures needed are
   in, and is green from then on.
4. Under **Download**, choose what to take:
   - **EML**: the ER file in XML.
   - **Results CSV**: the same votes as a CSV.
   - **Transmission package**.
   - **Transmission Report**.
   - **OVCS public key**: the receiver needs it to check the OVCS signature.

Every download of the EML, the CSV or the package is recorded before the file
is fetched. The record is kept in two places:

- **The package's log** gets a line naming the file, its SHA-256 and who
  downloaded it, for example
  `Downloaded all_servers.zip (SHA-256 E8D67AC4…) by Maria L. Santos`.
- **The election event's log** gets a `TransmissionFileDownloaded` entry, with
  the file, its SHA-256, the transaction id and the person.

Together they record the chain of custody. If the download cannot be recorded,
the page does not download the file.

### The results CSV

The CSV is generated with the EML and holds the same votes. Its columns are
`transaction_id, station_id, post, country, contest_id, contest, row,
candidate_id, candidate, party_id, votes`.

- **Candidates:** each contest has one `CANDIDATE` row per candidate.
- **Counts:** after its candidates, each contest has one row per count. The
  `row` column holds the EML's metric id:

  | Id | Count |
  | --- | --- |
  | `RV` | Registered voters |
  | `BB` | Ballots in the box |
  | `VB` | Valid ballots |
  | `IB` | Invalid ballots |
  | `VV` | Votes |
  | `UV` | Under votes |

### The Transmission Report

**Transmission Report** in the same menu generates a PDF for the Post (the
election) of the tally, with one page per country (area). Each page shows:

- the geographical region, the Post, the country, the voting center and the
  polling station's ID and name, from the Post's and the country's `miru:`
  annotations;
- the registered voters (`miru:registered-voters`), the voters whose valid
  ballot was counted in the country, and the turnout;
- for each destination of the country, whether the ER file was transmitted
  and received, with the time of each. A destination with manual receipt
  shows **Not Transmitted** and **Not Received**, since nothing is sent over
  the network;
- a signature box for each SBEI member of the country, with their name and
  role, three lines for watchers, and the results, report and system hashes
  with a QR code.

It needs the `transmission-report-generate` permission, and the tally
session must count the Post. If the election event has a Transmission
Report saved for the Post in **Reports**, its template is used; otherwise
the default template is.

## Checking the package on receipt

The receiving computer checks the package offline with `step-cli`. It needs
no network connection and no Java:

```bash
step-cli step verify-transmission-package all_servers.zip \
    --key ccs-manila-private-key.pem \
    --ovcs-key ovcs-public-key.pem \
    --ca event-ca.pem --certificates sbei-certificates.pem \
    --required-signatures 2 \
    --expect-sha256 "E8D67AC4 …"
```

It checks:

- that the package opens with the destination's private key;
- that the ER file's SHA-256 matches the one in the package and, when
  `--expect-sha256` is given, the one shown on the page;
- that the OVCS signature verifies with the OVCS public key;
- that each SBEI signature is valid, and that the SBEI's certificate chains to
  the event's CA (`--ca`, the root and intermediate certificates);
- that the signatures needed are present.

It prints the station, the transaction, the ER file's SHA-256, the signers and
the counts per contest. It exits with an error and the reason when any check
fails. It refuses, for example:

- an ER file that was changed and sealed again;
- a package whose SHA-256 was also replaced;
- a package missing an SBEI signature;
- a package with a changed byte;
- a package opened with another destination's key.

## The test (T1 to T8)

Run it on any test event with the sample configuration.

| Step | How | Passes when |
| --- | --- | --- |
| T1 Sample configuration | Load it; tally the Post | Every value the generator reads is present; the tally completes |
| T2 Generate | Generate the package for the country | The page shows the ER file's SHA-256, no Send, and Manual receipt with the key the package is sealed for |
| T3 Sign | Two SBEIs sign | The chip is green at 2 needed; the package is sealed again with both signatures |
| T4 Download | Download the package, the EML and the CSV | All three download; the log names each file, its SHA-256 and who downloaded it |
| T5 Check the files | Compare the XML and CSV with the election returns | The XML is EML 510 with the page's SHA-256; the CSV has the same votes |
| T6 Check on receipt | Run the verifier offline with the destination's key and the page's SHA-256 | It opens, the SHA-256 matches, and the OVCS and SBEI signatures are valid and chain to the event's CA |
| T7 Altered copies | Run the verifier on altered copies | Each one is refused |
| T8 Transmission Report | Generate and download the Post's Transmission Report | The PDF shows the configuration's Post, country, station, registered voters and destinations, the tally's counted voters and turnout, and the SBEI members |

Where the test runs:

- **CI:** on every pull request.
  - The backend journey `test_d_er_transmission_manual_receipt`
    (`scripts/e2e/journeys`) runs T2 to T7 on a full stack after the journeys'
    tally. `test_2c_transmission_sample_ballot` loads the ballot's part of the
    configuration before publication. `test_e_transmission_report` runs T8
    and keeps the PDF and its text in the output's `transmission` folder.
  - Windmill's tests check the sample configuration, the CSV, the sealing
    against Step's ECIES tool, and the verifier's refusals.
- **Staging:** run it before the post-qualification.
- **Post-qualification:** T2 to T5 are done live on the LAN with COMELEC's
  sample records. The recordings are for rehearsal only.

Keep the evidence of each run:

- the three files with their SHA-256;
- the page's log;
- the handover record: who carried which file, with which SHA-256, and when;
- the Step release and the configuration used.

## Not covered by manual receipt

These stay open until COMELEC names the receiver and its format:

- Sending to destinations over the network, with acknowledgements and
  automatic retry.
- Sending a copy of the encrypted ballots.
- Bandwidth requirements.
- The compatibility test with the canvassing system.
