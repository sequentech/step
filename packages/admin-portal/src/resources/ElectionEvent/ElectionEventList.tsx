// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Button, CreateButton, useGetList} from "react-admin"
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
import AddIcon from "@mui/icons-material/Add"
import PublishIcon from "@mui/icons-material/Publish"
import {
    CreateElectionEventProvider,
    useCreateElectionEventStore,
} from "@/providers/CreateElectionEventContextProvider"
import {CreateDataDrawer} from "@/components/election-event/create/CreateElectionEventDrawer"
import {ImportDataDrawer} from "@/components/election-event/import-data/ImportDataDrawer"

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

export const ElectionEventListContent: React.FC<ElectionEventListProps> = ({aside}) => {
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

    const {openCreateDrawer, openImportDrawer} = useCreateElectionEventStore()

    const {data, isLoading, refetch} = useGetList(
        "sequent_backend_election_event",
        {
            sort: {field: "created_at", order: "DESC"},
            filter: {
                tenant_id: tenantId,
                is_archived: isArchivedElectionEvents,
            },
        },
        {
            enabled: false,
        }
    )

    // Reload data when the path changes
    useEffect(() => {
        refetch()
    }, [pathname])

    // if data, we would be automatically redirected to the first election
    // event, so we should just show a process icon in the meantime
    useEffect(() => {
        if (data && data.length > 0) {
            const electionEventId = data[0].id ?? null
            if (electionEventId) {
                navigate("/sequent_backend_election_event/" + electionEventId)
            }
        } else {
            navigate("/sequent_backend_election_event")
        }
    }, [data])

    return (
        <CenteredBox>
            {isLoading ? (
                <CircularProgress />
            ) : (
                <>
                    <EmptyBox m={1}>
                        <Typography variant="h4" paragraph>
                            {t("electionEventScreen.error.noResult")}
                        </Typography>
                        {canCreateElections ? (
                            <>
                                <Typography variant="body1" paragraph>
                                    {t("common.resources.noResult.askCreate")}
                                </Typography>
                                <Box display="flex" gap={1}>
                                    <Button
                                        label={t("common.label.add")}
                                        startIcon={<AddIcon />}
                                        onClick={() => openCreateDrawer()}
                                    />
                                    <Button
                                        label={t("common.label.import")}
                                        startIcon={<PublishIcon />}
                                        onClick={() => openImportDrawer?.()}
                                    />
                                </Box>
                            </>
                        ) : null}
                    </EmptyBox>
                    <CreateDataDrawer />
                    <ImportDataDrawer
                        title="electionEventScreen.import.eetitle"
                        subtitle="electionEventScreen.import.eesubtitle"
                        paragraph={"electionEventScreen.import.electionEventParagraph"}
                    />
                </>
            )}
        </CenteredBox>
    )
}

export const ElectionEventList: React.FC<ElectionEventListProps> = (props) => (
    <CreateElectionEventProvider>
        <ElectionEventListContent {...props} />
    </CreateElectionEventProvider>
)
