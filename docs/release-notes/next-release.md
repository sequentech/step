<!--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Release NEXT

## ✨ Ask for Admin password for sensitive actions

This feature changes the behavior of some sensitive actions like starting an
election voting period or publishing a new publication of the ballot styles.

The way it works is by requiring gold level of authentication and for that the
user needs to re-authenticate.

### Keycloak: Migration to add `gold` level of authentication support

In the Admin Portal Realm:
1. Click `Realm Settings` in the sidebar
2. In the `General` tab, click `Add ACR to LoA Mapping`
3. Add two key-values pairs:
    - key: `silver`
      value: `1`
    - key: `gold`
      value: `2`
4. Click `Authentication` in the sidebar
5. Click `sequent browser blow` and ensure it has a new conditional subflow
   called `advanced / gold condition` with a required conndition of type
   `Condition - Level of Authentication` and value `2` and a Required
   `Password Form` step.