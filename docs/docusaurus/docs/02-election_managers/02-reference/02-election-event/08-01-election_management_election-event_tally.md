---
id: election_management_election_event_tally
title: Tally Ceremony
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->


## Introduction
The Tally section starts a tally ceremony, collects trustee authorizations, runs
mixing and threshold decryption, and displays the counted results.

## Closing Key Ceremony
During a tally, the selected quorum of wbraid trustees uses its locally retained
election shares to decrypt the shuffled ballots:

- **Start the runners:** Each participating trustee unlocks its native or browser
  runner with the same identity and protocol state used for key generation.
- **Authorize a manual tally:** Each trustee generates a public custody proof
  bound to this tally session and submits it through the ceremony wizard.
- **Mix and decrypt:** The runners verify ballot proofs and shuffle proofs,
  then publish verifiable partial decryptions and agree on the plaintext ballots.
- **Count results:** Velvet decodes the plaintexts and applies the configured
  counting rules, channels and weights.

Election shares remain with their trustees. Threshold decryption combines partial
decryptions without reconstructing or uploading the private key.

## Prerequisites for Tally
Before initiating the tally process:

1. **Opening Key Ceremony completed:** The joint public key has been verified and
   any required manual custody confirmations have completed.
2. **Voting phase concluded (optional):** Votes may or may not have been cast; starting a tally with no votes is technically possible but yields empty results.

> Automatic ceremonies skip manual custody confirmation. The trustee runners must
> still be active with their saved identities and protocol state to mix and decrypt.

## Starting the Tally Process
1. Navigate to the relevant **Election Event** in the Administration Portal.
2. Go to the **Tally** tab.
3. Click **Start Tally Ceremony**.
4. Select one or more Elections to include in this tally.
5. Confirm to proceed; trustees are notified to authorize a manual tally.
6. Once the required threshold of trustee authorizations is reached, the ceremony
   becomes **CONNECTED**. Select **Start Tally** to begin execution.
7. Keep the participating runners active until the tally completes. A success
   notification appears when complete.

## Trustee Custody Confirmation for Tally
1. Log in to the Administration Portal and open the relevant Election Event.
2. Go to the **Tally** tab.
3. Click the Key Action invitation or the key icon next to the Tally ceremony.
4. Select the Election to tally.
5. Use the public custody request saved from the key ceremony to create a new proof:

   ```sh
   trustee attest --request custody-request.json --trustee-config trustee.toml --tally-session-id <TALLY_SESSION_ID> --output custody-proof.json
   ```

   Browser trustees use **Create custody proof** with the tally session identifier
   on `/trustee`. Upload `custody-proof.json`, then complete any required approval
   signature. Keep private keys and protocol state on the trustee machine.
6. Repeat for enough trustees to reach the ceremony threshold. A proof for another
   tally, or an earlier key ceremony proof, cannot authorize this tally.
7. Once the ceremony is **CONNECTED**, the administrator selects **Start Tally**.

See [trustee setup and recovery](../../../07-developers/09-braid/braid_trustees_configuration.md)
and [independent public verification](../../../07-developers/09-braid/public-verification.md)
for checking the DKG and tally transcript without trustee keys.

## Reviewing Results
After tally completion, results become available under the **Tally** tab:

- **View Results**: Click the View Results button for each Election.
- **Columns**: Enable or disable columns as needed for clarity.
- **Elections Tally Progress**: Monitor status (e.g., “In Progress,” “Success”).
- **Logs**: Access detailed logs for auditing and troubleshooting.
- **General Information**: Displays tally date, total voter participation, etc.
- **Results and Participation**: Detailed breakdown by Areas, Contests, and Elections.

### General Information
- **Tally Date and Time**: When the tally was finalized.
- **Voter Participation**: Summary of how many eligible voters participated.

### Results & Participation Breakdown
- **By Election**: Results for each Election within the Event.
- **By Contest**: Results for each Contest across Elections.
- **By Area**: Results segmented by geographic or organizational divisions.
- **Participation Summary**:
  - **Eligible Voters**: Total number eligible to vote.
  - **Total Voters**: Number who cast a vote.
  - **Total Valid Votes**: Count of valid ballots, including blank ballots.
  - **Total Invalid Votes**: Count of invalid ballots, subdivided into:
    - **Explicitly Invalid Votes**: Null votes, spoiled ballots, protest actions.
    - **Implicitly Invalid Votes**: Invalid due to configuration (e.g., multiple selections where only one is allowed).
  - **Blank Votes**: Explicit and implicit blank ballots. Blank ballots are
    included in total valid votes.

### Candidate Results
For each Contest, display:
- **Option / Candidate Name**
- **Number of Votes**: Count of valid votes for that candidate/option.
- **Percentage of Votes**: Proportion relative to valid non-blank votes when the
  result is a candidate row.
- **Ranking / Position**: Placement among candidates by vote count.

> Note: According to the Election Event Weighted Voting Policy, if the policy is set to ‘Weighted Voting for Areas’, the tally will be calculated using the weights assigned to each area during the last publication prior to the tally.
