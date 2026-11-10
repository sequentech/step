// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext} from "react"
import {TenantContext} from "@/providers/TenantContextProvider"
import {offersBrowserTrustee} from "@/services/browserTrusteeAccess"
import {TrusteeDashboard} from "./TrusteeDashboard"

/** The route `/trustee`: the browser trustee page, only where the tenant offers it. */
export const TrusteeDashboardRoute: React.FC = () => {
    const {tenant} = useContext(TenantContext)
    return offersBrowserTrustee(tenant) ? <TrusteeDashboard /> : null
}
