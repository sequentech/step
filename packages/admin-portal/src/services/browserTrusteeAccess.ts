// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Sequent_Backend_Tenant} from "@/gql/graphql"
import {signedPackagesRequired} from "./configurationSigningPolicy"

/**
 * Whether the tenant offers the browser trustee page: the side-menu entry
 * "Trustee Dashboard" and the route `/trustee`. A tenant that requires signed
 * configuration packages runs its trustees on the server and authorizes custody
 * with the offline trustee tool, so it does not offer the page. A tenant without
 * the setting requires them, and until the tenant has loaded the page is not
 * offered either (see `signedPackagesRequired`).
 */
export const offersBrowserTrustee = (
    tenant: Pick<Sequent_Backend_Tenant, "settings"> | null | undefined
): boolean => !signedPackagesRequired(tenant)
