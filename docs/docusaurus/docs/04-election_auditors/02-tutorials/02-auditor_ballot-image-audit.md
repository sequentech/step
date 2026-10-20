---
id: auditor_ballot_image_audit
title: Ballot Image Audit (Paper Trail)
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The ballot images are a paper trail of the election: one image of each decrypted ballot. You can
count them by hand and compare the count with the results document.

## Before you start

- The tally status is `SUCCESS`.
- Your platform has the ballot images export. Without it, the results of the tally do not show
  **Generate Ballot Images**.
- The election administrator supervises the count. The ballot images stay inside the election
  organization.

:::danger WARNING
Do not copy or take the ballot images out of the supervised count. The images are grouped by
contest and area. In an area with few voters, a person can find how a voter voted.
:::

## What the images contain

The election administrator makes the images from the results of a tally. The platform makes one
folder for each election, contest and area, with one image for each ballot. Each image shows the
election, the contest, the area, the choices and whether the ballot is blank or invalid.

The **Ballot ID** on an image is a new number that the platform makes for the image. It is not
the ballot ID that the voter received, so you cannot find the ballot of a voter.

## Count the ballots

1. For each contest and area, count the images.
2. Make sure that the number of images is equal to the **Total votes counted** of that area in
   the results document.
3. Count the votes of each candidate on the images.
4. Compare your count with the results document.
5. Record the differences, if there are differences, and give them to the election
   administrator.

## More information

- [Election results verification](01-auditor_results-verification.md)
- [Prepare the Audit Pack](../../02-election_managers/03-procedures/08-audit-pack.md)
