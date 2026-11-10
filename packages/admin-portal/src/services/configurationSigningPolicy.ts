// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Sequent_Backend_Tenant} from "@/gql/graphql"

/**
 * Whether the tenant requires signed configuration packages. The tenant setting
 * `configuration_signing` is read as the server reads it (`signatures_required`
 * in sequent-core): only a setting that names the policy `optional` makes them
 * optional. A tenant without the setting, a setting that names no policy or
 * another one, and a tenant that has not loaded yet all require them.
 */
export const signedPackagesRequired = (
    tenant: Pick<Sequent_Backend_Tenant, "settings"> | null | undefined
): boolean => tenant?.settings?.configuration_signing?.policy !== "optional"
