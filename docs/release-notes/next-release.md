<!--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Release NEXT

    ## ✨ Admin Portal > Voters > required fields when create/edit a user

    ### User / Voter edit or create required fields

    - to define a field as required in users or voters screens we must go to keycloak profiles attributes:
        - for the users, at tenant level
        - for voters, at a election event level

    - the attribute can be marked as:
        - not required
        - required. In that case the are this levels of control:
            - admin, means required in the admin portal to a keycloak admin
            - user, means required for an admin portal admin
            - both

    - in FE, the field for the attribute is 
        - always required **if the keycloak attribute is required** no matter if there is defined for a user or an admin.  
        - not required **if the keycloak attribute is not required**

