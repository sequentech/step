// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useState} from "react"
import {Dialog, downloadUrl} from "@sequentech/ui-essentials"
import {
    Box,
    CircularProgress,
    MenuItem,
    Select,
    SelectChangeEvent,
    TextField,
    Typography,
} from "@mui/material"
import {
    CreateScheduledEventMutation,
    FetchDocumentQuery,
    Sequent_Backend_Election,
    Sequent_Backend_Election_Event,
    Sequent_Backend_Tally_Session,
    Sequent_Backend_Tally_Session_Execution,
    Sequent_Backend_Trustee,
} from "../gql/graphql"
import {useGetList, useGetOne, useRefresh} from "react-admin"
import {StyledChip} from "./StyledChip"
import {IconButton} from "@sequentech/ui-essentials"
import {faPlusCircle} from "@fortawesome/free-solid-svg-icons"
import {styled} from "@mui/material/styles"
import {useMutation, useQuery} from "@apollo/client"
import {CREATE_SCHEDULED_EVENT} from "../queries/CreateScheduledEvent"
import {ScheduledEventType} from "../services/ScheduledEvent"
import {useTenantStore} from "../providers/TenantContextProvider"
import {FETCH_DOCUMENT} from "@/queries/FetchDocument"
import {SettingsContext} from "@/providers/SettingsContextProvider"

const Horizontal = styled(Box)`
    display: flex;
    flex-direction: row;
    gap: 8px;
`

interface DownloadDocumentProps {
    documentId: string
    electionEventId: string
    onClose: () => void
}
function DownloadDocument({documentId, electionEventId, onClose}: DownloadDocumentProps) {
    const [tenantId] = useTenantStore()
    const {data: document} = useQuery<FetchDocumentQuery>(FETCH_DOCUMENT, {
        variables: {
            tenantId: tenantId,
            electionEventId: electionEventId,
            documentId: documentId,
        },
    })

    useEffect(() => {
        if (!document?.fetchDocument?.url) {
            return
        } else {
            console.log(`FF ${document?.fetchDocument?.url}`)
            downloadUrl(document?.fetchDocument?.url ?? "", "tally.tar.gz")
            onClose()
        }
    }, [document?.fetchDocument?.url])

    return <>Downloading</>
}

interface SelectElectionsProps {
    electionEvent: Sequent_Backend_Election_Event
    selectedElections: Array<Sequent_Backend_Election>
    onAddSelectedElection: (value: Sequent_Backend_Election) => void
}

export const SelectElections: React.FC<SelectElectionsProps> = ({
    electionEvent,
    selectedElections,
    onAddSelectedElection,
}) => {
    const [election, setElection] = useState<Sequent_Backend_Election | null>(null)
    const {data, total, isLoading, error} = useGetList("sequent_backend_election", {
        pagination: {page: 1, perPage: 10},
        sort: {field: "last_updated_at", order: "DESC"},
        filter: {
            tenant_id: electionEvent.tenant_id,
        },
    })

    const handleElectionChange = (event: SelectChangeEvent<Sequent_Backend_Election | null>) => {
        let id = event.target.value
        let election: Sequent_Backend_Election | undefined = (
            data as Array<Sequent_Backend_Election> | undefined
        )?.find((t) => t.id === id)
        if (election) {
            setElection(election)
        }
    }

    const onAddElection = () => {
        if (!election) {
            return
        }
        onAddSelectedElection(election)
        setElection(null)
    }

    return (
        <>
            <Box>
                {selectedElections.map((election) => (
                    <StyledChip label={election.name} key={election.id} />
                ))}
            </Box>
            <Horizontal>
                <Select
                    labelId="election-select-label"
                    id="election-select"
                    value={election}
                    renderValue={(value) => value?.name}
                    onChange={handleElectionChange}
                >
                    {data
                        ?.filter((election) => !selectedElections.find((t) => t.id === election.id))
                        .map((election) => (
                            <MenuItem key={election.id} value={election.id}>
                                {election.name}
                            </MenuItem>
                        ))}
                </Select>
                <IconButton icon={faPlusCircle} onClick={onAddElection} fontSize="24px" />
            </Horizontal>
        </>
    )
}

export interface StartTallyDialogProps {
    show: boolean
    handleClose: (val: boolean) => void
    electionEvent: Sequent_Backend_Election_Event
}

export const StartTallyDialog: React.FC<StartTallyDialogProps> = ({
    show,
    handleClose,
    electionEvent,
}) => {
    const [tenantId] = useTenantStore()
    const [selectedTrustees, setSelectedTrustees] = useState<Array<Sequent_Backend_Trustee>>([])
    const [selectedElections, setSelectedElections] = useState<Array<Sequent_Backend_Election>>([])
    const [createScheduledEvent] = useMutation<CreateScheduledEventMutation>(CREATE_SCHEDULED_EVENT)
    const [showProgress, setShowProgress] = useState(false)
    const [trustee, setTrustee] = useState<Sequent_Backend_Trustee | null>(null)
    const [tallySessionId, setTallySessionId] = useState<string | null>(null)
    const [documentId, setDocumentId] = useState<string | null>(null)
    const refresh = useRefresh()
    const {globalSettings} = useContext(SettingsContext)
    const {data, total, isLoading, error} = useGetList<Sequent_Backend_Trustee>(
        "sequent_backend_trustee",
        {
            pagination: {page: 1, perPage: 10},
            sort: {field: "last_updated_at", order: "DESC"},
            filter: {
                tenant_id: electionEvent.tenant_id,
            },
        }
    )
    const {data: tallySessionExecutions} = useGetList<Sequent_Backend_Tally_Session_Execution>(
        "sequent_backend_tally_session_execution",
        {
            pagination: {page: 1, perPage: 9999},
            sort: {field: "created_at", order: "DESC"},
            filter: {
                tenant_id: electionEvent.tenant_id,
                election_event_id: electionEvent.id,
                tally_session_id: tallySessionId ?? tenantId,
            },
        }
    )
    const {data: tallySession} = useGetOne<Sequent_Backend_Tally_Session>(
        "sequent_backend_tally_session",
        {
            id: tallySessionId ?? tenantId,
            meta: {
                tenant_id: tenantId,
            },
        },
        {
            refetchIntervalInBackground: true,
            refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
        }
    )

    useEffect(() => {
        if (
            !documentId &&
            tallySession?.is_execution_completed &&
            tallySessionExecutions?.[0]?.document_id
        ) {
            setDocumentId(tallySessionExecutions[0].document_id)
        }
    }, [documentId, tallySession?.is_execution_completed, tallySessionExecutions?.[0]?.document_id])

    const closeAll = () => {
        setShowProgress(false)
        handleClose(true)
    }

    if (isLoading || error) {
        return null
    }

    const startTally = async () => {
        setShowProgress(true)

        const {data, errors} = await createScheduledEvent({
            variables: {
                tenantId: tenantId,
                electionEventId: electionEvent.id,
                eventProcessor: ScheduledEventType.TALLY_ELECTION_EVENT,
                cronConfig: undefined,
                eventPayload: {
                    trustee_ids: selectedTrustees.map((t) => t.id),
                    election_ids: selectedElections.map((e) => e.id),
                },
            },
        })
        if (data?.createScheduledEvent?.id && !tallySessionId) {
            setTallySessionId(data?.createScheduledEvent?.id)
        }
        refresh()
        if (errors) {
            console.log(errors)
        }
    }

    const clickHandler = async (val: boolean) => {
        if (val) {
            try {
                await startTally()
            } catch (error) {
                console.log(`Error trying to start tally: ${error}`)
                setShowProgress(false)
                handleClose(false)
            }
        } else {
            handleClose(false)
        }
    }

    const handleTrusteeChange = (event: SelectChangeEvent<Sequent_Backend_Trustee | null>) => {
        let id = event.target.value
        let trustee: Sequent_Backend_Trustee | undefined = (
            data as Array<Sequent_Backend_Trustee> | undefined
        )?.find((t) => t.id === id)
        if (trustee) {
            setTrustee(trustee)
        }
    }

    const onAddTrustee = () => {
        if (!trustee) {
            return
        }
        setSelectedTrustees([...selectedTrustees, trustee])
        setTrustee(null)
    }

    const onAddSelectedElection = (election: Sequent_Backend_Election) => {
        setSelectedElections([...selectedElections, election])
    }

    return (
        <>
            {documentId ? (
                <DownloadDocument
                    documentId={documentId}
                    electionEventId={electionEvent.id}
                    onClose={closeAll}
                />
            ) : null}

            <Dialog
                handleClose={clickHandler}
                open={show}
                title="Tally Dialog"
                ok="OK"
                cancel="Cancel"
                variant="info"
            >
                <Typography variant="body1">Start Tally for Event</Typography>
                <Box>
                    {selectedTrustees.map((trustee) => (
                        <StyledChip label={trustee.name} key={trustee.id} />
                    ))}
                </Box>
                <Horizontal>
                    <Select
                        labelId="trustee-select-label"
                        id="trustee-select"
                        value={trustee}
                        renderValue={(value) => value?.name}
                        onChange={handleTrusteeChange}
                    >
                        {data
                            ?.filter(
                                (trustee) => !selectedTrustees.find((t) => t.id === trustee.id)
                            )
                            .map((trustee) => (
                                <MenuItem key={trustee.id} value={trustee.id}>
                                    {trustee.name}
                                </MenuItem>
                            ))}
                    </Select>
                    <IconButton icon={faPlusCircle} onClick={onAddTrustee} fontSize="24px" />
                </Horizontal>
                <SelectElections
                    electionEvent={electionEvent}
                    selectedElections={selectedElections}
                    onAddSelectedElection={onAddSelectedElection}
                />
                {showProgress ? <CircularProgress /> : null}
            </Dialog>
        </>
    )
}
