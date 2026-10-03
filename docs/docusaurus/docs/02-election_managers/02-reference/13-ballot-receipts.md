---
id: ballot_receipts
title: Ballot Receipts
sidebar_position: 13
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

With **Receipts signed by the ballot box** turned on, the ballot box receives, stores and signs a voter's ballot while the voter is still on the review screen. The **Ballot ID** shown there is computed from the voter-signed ballot and from the ballot box's signature, so a Ballot ID on the review screen means the ballot box has the ballot. The voter's device checks the signature before it shows the ID.

Casting is a second signature by the same procedure. The voter's device signs "cast this Ballot ID", the ballot box stores the cast and signs a **cast receipt**, and the device checks that signature before it shows the confirmation screen. A confirmation screen therefore means the ballot box has stored the vote as cast.

The setting is off by default. Election events without it keep computing the Ballot ID in the browser and send nothing to the backend until the voter casts.

---

## Enabling the setting

1. Open the **Election Event** and go to the **Data** tab.
2. Expand **Ballot receipts**.
3. Set **Receipts signed by the ballot box** to **Signed by the ballot box**.
4. Save, then publish the ballots again in the **Publish** tab.

Turning it on:

- Sets **Voter Signing Policy** to **With signature** and locks it. The ballot box receives only ballots their voter has signed.
- Creates the election event's **ballot box key** the next time the ballots are published, and publishes its public key in every ballot style.

The setting takes effect for voters when the ballots are published, and it cannot be changed in the Admin Portal once voting has started. Choose it before the voting period opens.

---

## What voters see

The review screen and its texts do not change.

| Moment | Review screen |
| --- | --- |
| The ballot is being sent | No Ballot ID box. **Cast ballot** is disabled. |
| The ballot box has stored and signed the ballot, and the device has checked the signature | The Ballot ID box shows the ballot box's Ballot ID. **Cast ballot** is enabled. |
| No answer, or the ballot box refuses the ballot | The error box shows the network or cast error text. No Ballot ID box. **Cast ballot** stays disabled. Going back to the ballot and continuing sends it again. |
| The ballot box's signature does not verify on the device | The error box shows the ballot hash error. The ballot cannot be cast from that device. |

Going back to edit the ballot makes a new ballot with a new Ballot ID. The earlier one stays received, is never cast and is never counted.

When the voter presses **Cast ballot**:

| Moment | Screen |
| --- | --- |
| The ballot box has stored the cast and signed its receipt, and the device has checked the signature | The confirmation screen. |
| No answer, or the ballot box refuses the cast | The review screen's error box shows the network or cast error text. The ballot stays received and not cast, and the voter can press **Cast ballot** again. |
| The cast receipt's signature does not verify on the device | The review screen's error box shows the ballot hash error, and no confirmation screen is shown. |

The key that signs the ballot and the cast is made for that ballot and lives only in the page's memory. If the election requires a fresh sign-in to cast, the device signs before it leaves the page and keeps only the signature. If the page is reloaded between review and cast, the voter returns to the ballot and it is sent again, with a new Ballot ID.

The confirmation screen, its link to **Locate your ballot** and the printed receipt show the same Ballot ID the voter saw at review. **Locate your ballot** accepts it in any case and with or without the hyphen.

Demo ballots and elections decided by acclamation send nothing to the ballot box. Telephone voting keeps casting directly, because there is no voter device to check a receipt.

---

## The Ballot ID

A Ballot ID is eight characters in two groups of four, for example `FTBE-MHRX`. It uses the digits and the capital letters without I, L, O and U, so it can be read out and typed without confusion. A typed ID is accepted in any case, with or without the hyphen, with O read as 0 and I or L read as 1.

It is unique in its election event. It is a short name for the ballot, not a secret: the ballot box's signature carries the proof.

---

## What the ballot box checks

Before it stores a ballot at review, the ballot box checks that:

- the voter may vote in the election, through a voting channel that is enabled and open;
- the hash sent with the ballot is the hash of the ballot;
- the proofs of the encrypted contests are valid;
- the ballot carries a voter signature, and it verifies;
- the ballot names the ballot style published for the voter's area and election, and carries that style's hash.

A ballot that fails a check is refused with a client error and is not stored. The ballot box signs only after it has stored the ballot, and answers only after the database commit. Receiving the same ballot again returns the same receipt.

---

## What the ballot box checks at the cast

The voter's device sends the election, the Ballot ID and the voter's Cast signature. It does not send the ballot again. The ballot box checks that:

- it has received a ballot from that voter under that Ballot ID, in that election;
- the Cast signature verifies against the voter key of that received ballot;
- the ballot has not been cast or audited;
- the election is open for the voter's channel, and the revote rules allow one more vote.

It then inserts the cast vote with the content of the received ballot, marks the received ballot as cast and signs the cast receipt, all in one transaction, and answers only after the commit. The cast vote's time is the receipt's cast time.

- Casting again with the same Cast signature returns the same receipt, so a lost answer can be retried.
- Another voter's Ballot ID, an unknown Ballot ID, an audited ballot, a ballot already cast with another signature and a Cast signature that does not verify are refused with a client error, and nothing is cast.
- A cast that sends the ballot itself (`insert_cast_vote`) is refused while receipts are on, except for telephone voting.

---

## For auditors: verifying a receipt

The ballot box key is an Ed25519 key, one per election event, separate from the key that signs the event's log. The ballot style publishes it as `ballot_box_key`:

- `public_key`: Base64 of the DER SubjectPublicKeyInfo;
- `key_id`: the first 8 bytes of SHA-256 of that DER encoding, in hexadecimal.

The ballot box signs the **Received statement**. Every field is preceded by its length as an 8-byte little-endian integer, in this order:

| Field | Bytes |
| --- | --- |
| Domain | `step/ballot-received/v1` |
| Tenant | The tenant's UUID, as text |
| Election event | The election event's UUID, as text |
| Election | The election's UUID, as text |
| Ballot hash | The hexadecimal hash of the encrypted ballot |
| Voter public key | The DER encoding of the voter's single-use key |
| Voter signature | The 64 bytes of the voter's signature over the ballot |
| Received time | UTC with milliseconds, as text: `2028-05-08T03:00:00.000Z` |
| Key ID | The ballot box key's `key_id`, as text |

The **Ballot ID** is the first 40 bits of SHA-512 over these fields, length-prefixed in the same way: the domain `step/ballot-id/v1`, the ballot hash, the voter public key, the voter signature, the received time, the key ID and the 64 bytes of the ballot box's signature. The 40 bits are written as eight characters of Crockford's base32.

The voter's single-use key signs the **Cast statement**, with the same length prefixes:

| Field | Bytes |
| --- | --- |
| Domain | `step/ballot-cast/v1` |
| Election | The election's UUID, as text |
| Ballot ID | The Ballot ID, as text: `FTBE-MHRX` |

The ballot box signs the **Cast receipt statement**:

| Field | Bytes |
| --- | --- |
| Domain | `step/ballot-cast-receipt/v1` |
| Election event | The election event's UUID, as text |
| Election | The election's UUID, as text |
| Ballot ID | The Ballot ID, as text |
| Received time | The received time of the Received statement |
| Cast time | UTC with milliseconds, as text |
| Key ID | The ballot box key's `key_id`, as text |
| Cast signature digest | The 32 bytes of SHA-256 of the voter's Cast signature |

Each statement has its own domain, so a signature made for one never verifies as another.

Changing any byte of the ballot, of either voter signature or of either ballot box signature changes the Ballot ID or breaks a signature. Shared test vectors are in `packages/sequent-core/tests/ballot_receipts.rs`.

Received ballots are stored in `sequent_backend.received_ballot` with their status: `received`, `cast` or `audited`. A cast ballot keeps its cast time, the voter's Cast signature and the ballot box's Cast receipt signature there. A cast vote names the received ballot it was cast from in `cast_vote.received_ballot_id` and keeps the receipt signature in `cast_vote.cast_receipt_signature`.
