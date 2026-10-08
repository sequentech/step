---
id: settings_countries
title: Countries (Country Blocking)
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **Countries** tab is in **Settings**. Its title is **Country Blocking**. It blocks access to
the voting portal from the countries that you select. The tenant administrator uses it.

The tab has two lists:

- The countries from which voters cannot sign in to vote.
- The countries from which voters cannot enroll (register).

The block applies to the internet address of the voter. The system finds the country from that
address.

:::note
The block works only when your installation uses the web firewall service that Sequent
configures for country blocking. Ask Sequent support if you are not sure.
:::

## How to open it

1. In the menu on the left, click **Settings**.
2. Click the **Countries** tab.

## Required permission

- To open **Settings**, your account must have the **Edit Tenant** and **View settings**
  permissions.
- To save the lists, your account must also have the **Edit Country Blocking Rules in
  Cloudflare** permission.

## Fields

| Field | Meaning | Default |
| --- | --- | --- |
| **Countries** (below "Choose below the countries you want to block voting from.") | The countries from which voters cannot sign in to the voting portal. | Empty: no country is blocked. |
| **Countries** (below "Choose below the countries you want to block enrollment from.") | The countries from which voters cannot enroll. | Empty: no country is blocked. |
| **Save** | Saves the two lists and updates the blocking rules. | |

Each **Countries** field accepts more than one country. Type a part of the country name, then
select the country in the list.

## Block countries

1. Open the **Countries** tab.
2. In the first **Countries** field, select each country to block for voting.
3. In the second **Countries** field, select each country to block for enrollment.
4. Click **Save**.

**Expected result:** the fields show the selected countries after you open the tab again.

:::caution CAUTION
Make sure that you do not block a country where your voters live. Blocked voters cannot sign
in or enroll, and the voting portal does not tell them the reason.
:::

## Remove a block

1. Open the **Countries** tab.
2. In the field, click the **x** on the country that you want to remove.
3. Click **Save**.

If a list is empty when you save, the system removes the blocking rule for that list.

## If there is a problem

| Problem | Action |
| --- | --- |
| The lists do not change after **Save**, or an error message shows. | Make sure that your account has the **Edit Country Blocking Rules in Cloudflare** permission. If it has, contact Sequent support. |
| Voters from a country that is not in the list cannot sign in. | Contact Sequent support. Other network rules can block access. |

**Related procedure:** [01-TENANT: Set up the tenant](../../../03-procedures/01-tenant.md).
