// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Generate a supported tenant-config archive for the COMELEC two-file import,
// in-browser, from the versioned public profile snapshot (meta#14593). The archive
// members and schema match the backend exporter/importer (export_tenant_config.rs /
// import_tenant_config):
//   - export_permissions-<tenant>.csv : role,permissions (the nine groups' realm roles);
//   - keycloak_admin-<tenant>.json    : realm display + localization only;
//   - tenant_configuration-<tenant>.csv (only when target state is supplied): the target
//     tenant row with the approved public setting display_name="COMELEC" merged into its
//     settings, preserving id/slug and all unrelated state. The Google service-account
//     key is stripped, matching the backend exporter, so no secret can land in the
//     archive. The profile itself carries no users, clients or secrets.

import JSZip from "jszip"
import profile from "./comelec-tenant-profile.json"

/** The canonical groups and their exact realm-role mappings, for preflight/preview. */
export const comelecGroups: ReadonlyArray<{name: string; realmRoles: string[]}> =
    profile.groups

/** Setting keys that hold secret custody material and must never be exported. */
const SECRET_SETTING_KEYS = ["gapi_key"]

/** The selected tenant's current row, as read by GetTenantState. */
export interface TenantState {
    id: string
    slug: string
    created_at: string
    updated_at: string
    labels: unknown
    annotations: unknown
    is_active: boolean
    voting_channels: unknown
    settings: Record<string, unknown> | null
    test: number | null
}

export interface TenantConfigPreflight {
    /** Each canonical group and how many realm roles it grants. */
    groups: Array<{name: string; roleCount: number}>
    /** The distinct realm roles the archive would assign, for a preflight check. */
    roles: string[]
    /** The realm display name the keycloak section applies. */
    realmDisplayName: string
    /** The tenant setting display_name the archive applies, when target state is supplied. */
    appliedTenantDisplayName?: string
    profileVersion: number
    keycloakSourceSha256: string
}

export interface GeneratedTenantConfig {
    file: File
    preflight: TenantConfigPreflight
}

const quoteCsv = (value: string): string =>
    /[",\n\r]/.test(value) ? `"${value.replace(/"/g, '""')}"` : value

const jsonCell = (value: unknown): string => JSON.stringify(value ?? null)

/**
 * Build the tenant_configuration CSV (header + one row) from the target tenant row,
 * merging the approved public `display_name` into its settings and preserving id/slug
 * and every unrelated field. The service-account key is stripped. Columns and encoding
 * match import_tenant's `process_record`: slug/timestamps are bare; labels, annotations,
 * voting_channels and settings are JSON; is_active/test are scalars; `test` is column 9.
 */
export function buildTenantConfigurationCsv(
    target: TenantState,
    displayName: string
): string {
    const settings: Record<string, unknown> = {...(target.settings ?? {})}
    for (const key of SECRET_SETTING_KEYS) delete settings[key]
    settings.display_name = displayName
    const header = [
        "id",
        "slug",
        "created_at",
        "updated_at",
        "labels",
        "annotations",
        "is_active",
        "voting_channels",
        "settings",
        "test",
    ]
    const row = [
        target.id,
        target.slug,
        target.created_at,
        target.updated_at,
        jsonCell(target.labels),
        jsonCell(target.annotations),
        String(target.is_active),
        jsonCell(target.voting_channels),
        jsonCell(settings),
        target.test === null || target.test === undefined ? "" : String(target.test),
    ]
    return `${header.join(",")}\n${row.map(quoteCsv).join(",")}\n`
}

/**
 * Build the tenant-config archive and a preflight summary of exactly what it would
 * apply. When `targetTenant` is supplied the archive includes the tenant_configuration
 * member (applying the approved display_name while preserving the target); otherwise it
 * carries only roles + keycloak display/localization.
 */
export async function buildComelecTenantConfig(
    tenantId: string,
    targetTenant?: TenantState
): Promise<GeneratedTenantConfig> {
    // export_permissions: header `role,permissions`; one row per group, the group's
    // realm roles joined by `|` (read_roles_config_file splits on `|`).
    const lines = ["role,permissions"]
    for (const group of profile.groups) {
        lines.push(`${quoteCsv(group.name)},${quoteCsv(group.realmRoles.join("|"))}`)
    }
    const permissionsCsv = lines.join("\n") + "\n"

    // keycloak_admin: import_tenant_config applies only displayName + localizationTexts.
    const keycloakAdmin = {
        displayName: profile.realm.displayName,
        displayNameHtml: profile.realm.displayNameHtml,
        localizationTexts: profile.realm.localizationTexts,
    }

    const zip = new JSZip()
    zip.file(`export_permissions-${tenantId}.csv`, permissionsCsv)
    zip.file(`keycloak_admin-${tenantId}.json`, JSON.stringify(keycloakAdmin, null, 2))
    let appliedTenantDisplayName: string | undefined
    if (targetTenant) {
        appliedTenantDisplayName = profile.tenant.displayName
        zip.file(
            `tenant_configuration-${tenantId}.csv`,
            buildTenantConfigurationCsv(targetTenant, appliedTenantDisplayName)
        )
    }
    const blob = await zip.generateAsync({type: "blob", compression: "DEFLATE"})
    const file = new File([blob], `export-tenant-config-${tenantId}.zip`, {
        type: "application/zip",
    })

    const roles = Array.from(
        new Set(profile.groups.flatMap((group) => group.realmRoles))
    ).sort()
    return {
        file,
        preflight: {
            groups: profile.groups.map((group) => ({
                name: group.name,
                roleCount: group.realmRoles.length,
            })),
            roles,
            realmDisplayName: profile.realm.displayName,
            appliedTenantDisplayName,
            profileVersion: profile.version,
            keycloakSourceSha256: profile.provenance.keycloakSha256,
        },
    }
}
