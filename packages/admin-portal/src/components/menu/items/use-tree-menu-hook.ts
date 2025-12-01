// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useTenantStore} from "@/providers/TenantContextProvider"
import {FETCH_ELECTION_EVENTS_TREE} from "@/queries/GetElectionEventsTree"
import {useQuery} from "@apollo/client"
import {useContext} from "react"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {SettingsContext} from "@/providers/SettingsContextProvider"

export function useTreeMenuData(isArchivedElectionEvents: boolean) {
    const [tenantId] = useTenantStore()
    const {globalSettings} = useContext(SettingsContext)

    return useQuery(FETCH_ELECTION_EVENTS_TREE, {
        variables: {
            tenantId: tenantId,
            isArchived: isArchivedElectionEvents,
        },
        pollInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
    })
}

export function useActionPermissions() {
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)

    /**
     * election event
     */
    const canCreateElectionEvent = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.ELECTION_EVENT_CREATE
    )
    const canWriteElectionEvent = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.ELECTION_EVENT_WRITE
    )
    const canDeleteElectionEvent = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.ELECTION_EVENT_DELETE
    )
    const canReadElectionEvent = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.ELECTION_EVENT_READ
    )
    const canArchiveElectionEvent = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.ELECTION_EVENT_ARCHIVE
    )

    /**
     * election
     */
    const canWriteElection = authContext.isAuthorized(true, tenantId, IPermissions.ELECTION_WRITE)
    const canReadElection = authContext.isAuthorized(true, tenantId, IPermissions.ELECTION_READ)
    const canCreateElection = authContext.isAuthorized(true, tenantId, IPermissions.ELECTION_CREATE)
    const canDeleteElection = authContext.isAuthorized(true, tenantId, IPermissions.ELECTION_DELETE)

    /**
     * contest
     */
    const canWriteContest = authContext.isAuthorized(true, tenantId, IPermissions.CONTEST_WRITE)
    const canReadContest = authContext.isAuthorized(true, tenantId, IPermissions.CONTEST_READ)
    const canCreateContest = authContext.isAuthorized(true, tenantId, IPermissions.CONTEST_CREATE)
    const canDeleteContest = authContext.isAuthorized(true, tenantId, IPermissions.CONTEST_DELETE)

    /**
     * candidate
     */
    const canWriteCandidate = authContext.isAuthorized(true, tenantId, IPermissions.CANDIDATE_WRITE)
    const canReadCandidate = authContext.isAuthorized(true, tenantId, IPermissions.CANDIDATE_READ)
    const canCreateCandidate = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.CANDIDATE_CREATE
    )
    const canDeleteCandidate = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.CANDIDATE_DELETE
    )

    return {
        canCreateElectionEvent,
        canReadElectionEvent,
        canDeleteElectionEvent,
        canWriteElectionEvent,
        canArchiveElectionEvent,
        canWriteElection,
        canReadElection,
        canCreateElection,
        canDeleteElection,
        canWriteContest,
        canReadContest,
        canCreateContest,
        canDeleteContest,
        canWriteCandidate,
        canReadCandidate,
        canCreateCandidate,
        canDeleteCandidate,
    }
}
