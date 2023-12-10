// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {CreateButton, useGetList} from "react-admin"
import React, {ReactElement, useContext, useEffect} from "react"

import {useLocation, useNavigate} from "react-router-dom"
import {CircularProgress, Typography} from "@mui/material"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {styled} from "@mui/material/styles"
import {Box} from "@mui/material"
import {useTranslation} from "react-i18next"
import {useAtom} from "jotai"
import archivedElectionEventSelection from "@/atoms/archived-election-event-selection"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"

const EmptyBox = styled(Box)`
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    width: 100%;
`
const CenteredBox = styled(Box)`
    display: flex;
    justify-content: center;
    align-items: center;
    height: 100%;
    text-align: center;
`

export interface ElectionEventListProps {
    aside?: ReactElement
}

export const ElectionEventList: React.FC<ElectionEventListProps> = ({aside}) => {
    const {t} = useTranslation()
    const navigate = useNavigate()
    const {pathname} = useLocation()
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)
    const [isArchivedElectionEvents] = useAtom(archivedElectionEventSelection)
    const canCreateElections = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.ELECTION_EVENT_WRITE
    )

    const {data, isLoading} = useGetList("sequent_backend_election_event", {
        sort: {field: "created_at", order: "DESC"},
        filter: {
            tenant_id: tenantId,
            is_archived: isArchivedElectionEvents,
        },
    })

    // Navigate to the first election event found, if any
    useEffect(() => {
        if (data && data.length > 0) {
            const electionEventId = data[0].id ?? null
            if (electionEventId) {
                navigate("/sequent_backend_election_event/" + electionEventId)
            }
        } else if (pathname != "/sequent_backend_election_event/") {
            navigate("/sequent_backend_election_event/")
        }
    })

    const Empty = (
        <EmptyBox m={1}>
            <Typography variant="h4" paragraph>
                {t("electionEventScreen.error.noResult")}
            </Typography>
            {canCreateElections ? (
                <>
                    <Typography variant="body1" paragraph>
                        {t("common.resources.noResult.askCreate")}
                    </Typography>
                    <CreateButton />
                </>
            ) : null}
        </EmptyBox>
    )

    // if data, we would be automatically redirected to the first election
    // event, so we should just show a process icon in the meantime
    return <CenteredBox>{isLoading ? <CircularProgress /> : Empty}</CenteredBox>
}
