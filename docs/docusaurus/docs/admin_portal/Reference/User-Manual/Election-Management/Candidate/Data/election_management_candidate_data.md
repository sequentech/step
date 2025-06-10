<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

---
id: election_management_candidate_data
title: Data
---

The Candidate Data tab allows administrators to configure and manage details for a specific candidate within a Contest. Many settings parallel those in other data tabs (e.g., Election Event, Election, Contest), adapted for candidate-specific context. Below are the main sections and their details.

## Sections Overview
- **General**: Basic details and eligibility toggles.
- **Type**: Attributes affecting ballot behavior (invalid, blank, list, write-in).
- **Image**: Upload a picture/icon for the candidate.

---

## General
Configure fundamental details and voting eligibility for this candidate.

- **Name**  
  - Enter the official name of the candidate.  
  - Supports multilingual names if the contest/election is multilingual.
- **Alias (optional)**  
  - System-only alias for internal display or sorting.
- **Description or Bio (optional)**  
  - Brief description or notes about the candidate (if supported).
- **Enabled / Disabled Toggle**  
  - **Enabled**: Candidate appears on ballots; voters may select.  
  - **Disabled**: Candidate is hidden/greyed out; cannot be selected by voters.  
  - Use this to deactivate a candidate without deleting their record.
- **Additional Fields (if available)**  
  - Any custom metadata fields shown to administrators (e.g., affiliation, qualifications).

---

## Type
Define special roles or behaviors that affect how votes for this candidate are counted.

- **Invalid Vote**  
  - Marks this entry as an explicitly invalid vote option (e.g., “Null Vote,” “Spoil Ballot”).  
  - When selected by a voter, counts as an explicitly invalid vote in tally summaries.
- **Blank Vote**  
  - Marks this entry as a blank vote option.  
  - When selected, counts as a valid vote (i.e., a conscious choice to cast a blank ballot).
- **Category/List**  
  - Marks this entry as a list or party grouping rather than an individual candidate.  
  - Behavior may differ in list-based contests (e.g., list vote counting, seat allocation).  
  - If enabled, ensure contest settings (Ballot Design) support list-based voting.
- **Write-In**  
  - Indicates this entry represents a write-in candidate.  
  - Voters can submit free-text entries if write-in is allowed; system may aggregate write-in entries under this profile or handle per input.  
  - Ensure contest permits write-ins before enabling.

> **Note on Multiple Types**  
> Only one type should apply per candidate entry unless the system supports combined roles (e.g., a list that also allows blank votes is unlikely). Validate that settings do not conflict (e.g., a candidate cannot be both “Invalid Vote” and a normal candidate).

---

## Image
Upload or manage a picture/icon for the candidate, used in administration screens and voter-facing ballots (if supported).

- **Upload**  
  - Drag and drop a PNG (or other supported format) file.  
  - The system may enforce size or aspect ratio guidelines; follow any displayed recommendations.
- **Preview**  
  - After upload, verify the image displays correctly (e.g., not distorted, properly cropped).
- **Usage**  
  - Candidate image may appear alongside name on ballot pages, in result summaries, or in admin lists.
- **Replace or Remove**  
  - To update the image, upload a new file.  
  - To remove, use the delete/remove action if available; ensure fallback behavior (e.g., default icon) is acceptable.

---

## Workflow & Tips
- **Ordering**  
  - The order of candidate entries often follows the contest’s Ballot Design settings (random, alphabetical, custom). After adding/editing candidates, verify ordering as needed.
- **Disabling vs. Deletion**  
  - Use the Enabled/Disabled toggle to temporarily remove a candidate from ballots without losing historical data.  
  - Only delete a candidate if certain they should never appear; note potential audit implications.
- **Type Consistency**  
  - Ensure only one candidate entry is marked as “Blank Vote” or “Invalid Vote” unless multiple invalid options are supported.  
  - For list-based contests, confirm list candidates are configured correctly and contest settings align.
- **Multilingual Support**  
  - If the election is multilingual, provide translations for candidate names and any descriptions.  
  - Verify how write-in or invalid vote labels appear across languages.
- **Image Best Practices**  
  - Use clear, recognizable images or icons.  
  - Keep file sizes reasonable to avoid performance issues.
- **Validation**  
  - After saving changes, preview the ballot (if preview feature exists) to confirm candidate appears as intended or is hidden when disabled.
- **Audit Considerations**  
  - Changing a candidate’s type (e.g., marking as invalid) impacts result interpretation; apply changes before publishing ballots.  
  - Document any changes to invalid/blank vote options for transparency.


