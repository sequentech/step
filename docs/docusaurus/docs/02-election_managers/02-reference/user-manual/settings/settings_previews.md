---
id: settings_previews
title: External Previews
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->


The External Previews table provides a record of preview URLs generated via external API requests.
This page serves as a reference for administrators to track submission history, verify the requester's identity, and access the source ballot style document used for each generation.

The table consists of three primary columns:

* **Requested By:** Identifies the user or system account that initiated the preview request.
* **URL:** Provides a direct link to view the rendered ballot in the Voting Portal.
* **Document**: Contains a download button to retrieve the raw ballot style file.

### Key Actions

#### 1. Viewing the Preview: 
Clicking the link in the URL column opens the Voting Portal, loaded with the election event data contained in the source ballot style document. This allows for a visual verification of the layout as the voter will see it

#### 2. Downloading the Ballot Style Document:
In the Document column, use the download button to retrieve the raw ballot style file.
 This is used to verify the data integrity behind a preview, as it downloads the specific file version used to generate that particular preview instance.
