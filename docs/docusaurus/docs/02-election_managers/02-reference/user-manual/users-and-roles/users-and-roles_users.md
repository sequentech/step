---
id: users-and-roles_users
title: Users
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->




## Export administrative users

The **Users and roles** export includes a `group_name` column containing each
account's actual Keycloak group name. This is a membership, not a user-profile
attribute. Ordinary election voter exports do not add this column or read
administrative memberships.

An account without a group has an empty cell. If an account belongs to multiple
groups, export stops with an error instead of silently choosing one or producing
a value that the single-group importer cannot interpret. Export never changes
memberships. Resolve the intended grouping separately before using the file as
an import source.
