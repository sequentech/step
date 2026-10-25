// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Generate a supported tenant-config archive for the COMELEC two-file import,
// in-browser, from the versioned public profile snapshot (meta#14593). The archive
// members and schema match the backend exporter/importer (see export_tenant_config.rs
// and import_tenant_config): `export_permissions-<tenant>.csv` (role,permissions) and
// `keycloak_admin-<tenant>.json` (display + localization only). No `tenant_configuration`
// member is produced, so the target tenant's id/slug/settings are preserved; the
// archive is imported with include_tenant=false. No secrets, users or client config are
// present in the profile, so none can leak into the archive.

import JSZip from "jszip"
import profile from "./comelec-tenant-profile.json"

/** The canonical groups and their exact realm-role mappings, for preflight/preview. */
export const comelecGroups: ReadonlyArray<{name: string; realmRoles: string[]}> =
    profile.groups

export interface TenantConfigPreflight {
    /** Each canonical group and how many realm roles it grants. */
    groups: Array<{name: string; roleCount: number}>
    /** The distinct realm roles the archive would assign, for a preflight check. */
    roles: string[]
    /** The realm display name the keycloak section applies. */
    realmDisplayName: string
    profileVersion: number
    keycloakSourceSha256: string
}

export interface GeneratedTenantConfig {
    file: File
    preflight: TenantConfigPreflight
}

const quoteCsv = (value: string): string =>
    /[",\n\r]/.test(value) ? `"${value.replace(/"/g, '""')}"` : value

/**
 * Build the tenant-config archive and a preflight summary of exactly what it would
 * apply (the groups, the distinct roles, and the realm display name), so the caller
 * can preview and check against the target realm before any mutation.
 */
export async function buildComelecTenantConfig(
    tenantId: string
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
    const blob = await zip.generateAsync({
        type: "blob",
        compression: "DEFLATE",
    })
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
            profileVersion: profile.version,
            keycloakSourceSha256: profile.provenance.keycloakSha256,
        },
    }
}
