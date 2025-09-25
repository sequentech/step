---
id: election_management_election_voters
title: Voters
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->


# Voter Assignment in an Election

This section explains how to view and assign voters to a specific Election. Note that voters are created and managed at the Election Event level; assigning them to an Election happens indirectly via Areas and Contests.

## Overview

- **Voters are not created per Election**: Voter records exist at the Election Event level.
- **Assignment is indirect**: To include a voter in an Election, you assign them to an Area that is linked to one or more Contests within that Election.
- **Preconditions**: You must have at least one Contest defined for the Election before assigning voters.

## Key Concepts

1. **Election Event Voters**  
   - This tab shows all voters registered for the overall Election Event.
   - At this stage, a voter may have no Area or Contest assignment.

2. **Area**  
   - Represents a geographic or organizational division within the Election Event.
   - Contests are linked to Areas; voters assigned to an Area gain access to the Contests associated with that Area (i.e., the Elections within that Event).

3. **Contest**  
   - A contest belongs to an Election within the Election Event.
   - Contests must exist before voters can be effectively assigned to participate in them.

4. **Assignment Flow**  
   1. Create or verify the Contest in the Election Event.
   2. Create an Area (or verify existing Areas) in the Election Event.
   3. Link the Contest to the Area.
   4. Assign the voter (from Election Event Voters) to the Area.
   5. As a result, the voter is now part of the Election(s) associated with that Area and Contest.

## Detailed Steps: Assign a Voter to an Election

Follow these steps once you have defined Contests in your Election Event:

1. **Ensure the Contest Exists**  
   - In the Election Event, confirm that the desired Contest is created under the relevant Election.

2. **Check Current Voter Assignment**  
   - Navigate to the **Voters** tab in the Election Event.  
   - Observe that the voter record exists but has no Area assignment (and thus is not yet included in any specific Election).

3. **Create or Select an Area**  
   - Go to the **Areas** tab in the Election Event.  
   - If no suitable Area exists, create a new Area (e.g., “District A”).  
   - If Areas exist, choose the relevant one.

4. **Link Contest to Area**  
   - In the Areas list, select the **Edit** action for the chosen Area.  
   - In the Area edit screen, assign the appropriate Contest(s) to this Area.  
     - This links those Contest(s) (and their parent Election) to the Area.
   - Save changes.

5. **Assign Voter to the Area**  
   - Return to the **Voters** tab in the Election Event.  
   - Locate the voter record you wish to include.  
   - Select **Edit** (or “Assign”) for that voter.  
   - In the voter edit screen, set the Area field to the one just configured.  
   - Save changes.

6. **Result**  
   - The voter is now assigned to the Area linked to the Contest/Election.  
   - In the context of this specific Election: the voter will appear under that Election’s Voter list (via the Area assignment).

## Example Walkthrough

1. **Election Event Voters**  
   - Open the Election Event and go to **Voters**.  
   - Notice the voter “Alice Example” exists but shows no Area.

2. **Create Area**  
   - Go to **Areas**. Click **Add Area**.  
   - Name: “Downtown District”. Save.

3. **Link Contest to Area**  
   - In the Areas list, click **Edit** next to “Downtown District”.  
   - Under “Assigned Contests”, select “Mayor Race” (example Contest).  
   - Save.

4. **Assign Voter**  
   - Return to **Voters**. Click **Edit** next to “Alice Example”.  
   - In the “Area” dropdown, choose “Downtown District”.  
   - Save.

5. **Verify**  
   - Navigate to the specific Election (e.g., “Mayor Election”) and view its Voters.  
   - “Alice Example” now appears as an eligible voter for that Election.

## Notes & Tips

- If an Area is linked to multiple Contests/Elections, assigning a voter to that Area enrolls them in all linked Contests.  
- To exclude a voter from a particular Contest, do not link that Contest to the Area, or assign the voter to a different Area.  
- To manage multiple Contests per Area, use the Area’s edit screen to add or remove Contests as needed.  
- Changes take effect immediately; after assignment, the voter can access the Voting Portal for that Election.  
- If you later remove the Contest from the Area or reassign the voter to a different Area, their Election access adjusts accordingly.

---

