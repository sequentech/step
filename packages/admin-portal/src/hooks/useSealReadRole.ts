// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useContext} from "react"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"

/** The Hasura roles that may read ballot box seals, in the order they're tried. */
export const SEAL_READ_ROLES: IPermissions[] = [
    IPermissions.ELECTION_DASHBOARD_TAB,
    IPermissions.TALLY_READ,
    IPermissions.ADMIN_USER,
]

/** The first role of `SEAL_READ_ROLES` the user has, or null when none. */
export const sealReadRole = (hasRole?: (role: string) => boolean): IPermissions | null =>
    SEAL_READ_ROLES.find((role) => hasRole?.(role)) ?? null

/**
 * A role the signed-in user has that may read ballot box seals (VOTE-FREEZE),
 * so a page outside the Dashboard (deleting, the event's incident banner) can
 * read them; null when the user has none and the seals can't be read.
 */
export const useSealReadRole = (): IPermissions | null => {
    const {hasRole} = useContext(AuthContext)
    return sealReadRole(hasRole)
}
