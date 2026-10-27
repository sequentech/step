// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {DocumentNode, Kind, visit} from "graphql"

const settingsFields = new Set([
    "id",
    "tenant_id",
    "election_event_id",
    "is_archived",
    "administration_settings",
    "presentation",
    "__typename",
    "aggregate",
    "count",
    "nodes",
    "returning",
    "affected_rows",
])
const settingsRoot =
    /^(?:(?:insert|update|delete)_)?sequent_backend_(?:election_event|election)(?:_by_pk|_aggregate)?$/

/** Administrators receive a server-generated security-settings projection.
 * Aliasing it keeps existing settings forms usable without requesting election
 * data that their Hasura role cannot read. The database owns the access check. */
export function administratorSettingsQuery(document: DocumentNode): DocumentNode {
    return visit(document, {
        Field(node, _key, _parent, _path, ancestors) {
            const inSettings = ancestors.some(
                (ancestor) =>
                    !Array.isArray(ancestor) &&
                    ancestor.kind === Kind.FIELD &&
                    settingsRoot.test(ancestor.name.value)
            )
            if (!inSettings) return
            if (!settingsFields.has(node.name.value)) return null
            if (
                node.name.value === "presentation" ||
                node.name.value === "administration_settings"
            ) {
                return {
                    ...node,
                    alias: node.alias ?? {kind: Kind.NAME, value: "presentation"},
                    name: {kind: Kind.NAME, value: "administration_settings"},
                }
            }
        },
    })
}
