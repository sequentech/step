---
id: election_management_election_event_tally
title: Tally Ceremony
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->


## Introduction
The Tally section covers the essential procedures required to consolidate resultd after voting has concluded. Election Board members are guided through starting the tally ceremony, verifying key fragments, running the tally, and reviewing results. By following these instructions, the post-election phase is handled with integrity and precision, ensuring that results are verified and published correctly and that the process remains transparent and trustworthy.

## Closing Key Ceremony
During the Closing Key Ceremony for an Election Event, trustees use their cryptographic key fragments to decrypt the final tally:

- **Retrieve stored key fragments:** Each trustee securely retrieves their previously backed-up fragment of the private key.
- **Verify integrity and functionality:** Trustees perform cryptographic tests or audits to confirm each fragment is unaltered and functional.
- **Decrypt encrypted votes:** Trustees combine fragments (meeting the threshold) to decrypt the accumulated encrypted voting data, ensuring accurate and transparent results.
- **Secure key storage:** After decryption, key fragments are securely stored again to maintain confidentiality and authenticity of the election outcome.

This ceremony ensures that only a quorum of trustees can reconstruct the private key for decryption, preserving security throughout the election lifecycle.

## Prerequisites for Tally
Before initiating the tally process:

1. **Opening Key Ceremony completed:** All key fragments have been generated and verified.
2. **Voting phase concluded (optional):** Votes may or may not have been cast; starting a tally with no votes is technically possible but yields empty results.

> Note: According to the Election Event Keys/Tally Ceremonies Policy, if the Key Ceremony is set to 'Automatic,' the Tally Ceremony will also be automated, and no trustee action is required.

## Starting the Tally Process
1. Navigate to the relevant **Election Event** in the Administration Portal.
2. Go to the **Tally** tab.
3. Click **Start Tally Ceremony**.
4. Select one or more Elections to include in this tally.
5. Confirm to proceed; the system will notify trustees to verify their key fragments.
6. Trustees verify their fragments as prompted; once the threshold of fragments is verified, the tally begins automatically.
7. Wait for the system to finalize the tally. A success notification appears when complete.

## Trustee Key Verification for Tally
1. Log in to the Administration Portal and open the relevant Election Event.
2. Go to the **Tally** tab.
3. Click the Key Action invitation or the key icon next to the Tally ceremony.
4. Select the Election to tally.
5. Drag and drop or upload your encrypted key fragment for verification.
6. Repeat for each trustee until the threshold is reached.
7. Once all required fragments are verified, click **Start Tally** to begin decryption and tallying.

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
  - **Total Valid Votes**: Count of valid ballots.
  - **Total Invalid Votes**: Count of invalid ballots, subdivided into:
    - **Explicitly Invalid Votes**: Null votes, spoiled ballots, protest actions.
    - **Implicitly Invalid Votes**: Invalid due to configuration (e.g., multiple selections where only one is allowed).
  - **Blank Votes**: Ballots cast with no selection (counted as valid in many systems).

### Candidate Results
For each Contest, display:
- **Option / Candidate Name**
- **Number of Votes**: Count of valid votes for that candidate/option.
- **Percentage of Votes**: Proportion relative to total valid votes.
- **Ranking / Position**: Placement among candidates by vote count.

> Note: According to the Election Event Weighted Voting Policy, if the policy is set to ‘Weighted Voting for Areas’, the tally will be calculated using the weights assigned to each area during the last publication preior to the tally.

## Summary
By following this guide, administrators ensure the tally ceremony is conducted securely, transparently, and accurately. Key fragments are verified by trustees, encrypted ballots are decrypted correctly, and results are reviewed with full auditing capabilities. This maintains the integrity and trustworthiness vital for credible elections.
