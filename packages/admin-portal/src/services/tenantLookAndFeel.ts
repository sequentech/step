// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {IHelpLink, ITenantSettings, ITenantTheme} from "@sequentech/ui-core"
import type {Sequent_Backend_Tenant} from "@/gql/graphql"
import {signedPackagesRequired} from "./configurationSigningPolicy"

type TenantLook = Pick<Sequent_Backend_Tenant, "settings" | "annotations"> | null | undefined

/**
 * Whether the admin portal shows the tenant's logo (`annotations.logo_url`), applies its
 * custom CSS (`annotations.css`) and offers its help links (`settings.help_links`), and
 * whether Settings, Look & Feel offers the fields that set them. A tenant that requires
 * signed configuration packages keeps the staff screens as built, with the built-in
 * logo, and so does a tenant without the setting or one that has not loaded (see
 * `signedPackagesRequired`). The display name is not affected.
 */
export const staffScreensCustomizable = (
    tenant: Pick<Sequent_Backend_Tenant, "settings"> | null | undefined
): boolean => !signedPackagesRequired(tenant)

/** The address of the tenant's logo for the admin portal's screens, or none (the built-in logo). */
export const adminPortalLogoUrl = (tenant: TenantLook): string | undefined => {
    const url = staffScreensCustomizable(tenant)
        ? (tenant?.annotations as ITenantTheme | undefined)?.logo_url
        : undefined
    return typeof url === "string" && url !== "" ? url : undefined
}

/** The tenant's custom CSS for the admin portal's screens, or none. */
export const adminPortalCustomCss = (tenant: TenantLook): string => {
    const css = staffScreensCustomizable(tenant)
        ? (tenant?.annotations as ITenantTheme | undefined)?.css
        : undefined
    return typeof css === "string" ? css : ""
}

/** The tenant's help links for the side menu, or none. */
export const adminPortalHelpLinks = (tenant: TenantLook): Array<IHelpLink> => {
    const links = staffScreensCustomizable(tenant)
        ? (tenant?.settings as ITenantSettings | undefined)?.help_links
        : undefined
    return Array.isArray(links) ? links : []
}
