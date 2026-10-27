-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
-- Include inherited groups: changing a parent changes a member's effective permissions.
WITH RECURSIVE actor AS (
    SELECT u.id, u.realm_id FROM user_entity u
    JOIN realm r ON r.id = u.realm_id
    WHERE u.id = $1 AND r.name = $2
), effective_groups AS (
    SELECT g.id, g.parent_group, g.realm_id
    FROM actor a JOIN user_group_membership m ON m.user_id = a.id
    JOIN keycloak_group g ON g.id = m.group_id AND g.realm_id = a.realm_id
    UNION
    SELECT parent.id, parent.parent_group, parent.realm_id
    FROM effective_groups child
    JOIN keycloak_group parent ON parent.id = child.parent_group
        AND parent.realm_id = child.realm_id
)
SELECT EXISTS (SELECT 1 FROM actor) AS actor_present,
       EXISTS (SELECT 1 FROM effective_groups WHERE id = $3) AS affects_actor
