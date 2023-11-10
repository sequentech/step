// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
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
    Sequent_Backend_Election,
    Sequent_Backend_Election_Event,
    Sequent_Backend_Trustee,
} from "../gql/graphql"
import {useGetList, useRefresh} from "react-admin"
import {StyledChip} from "./StyledChip"
import {IconButton} from "@sequentech/ui-essentials"
import {faPlusCircle} from "@fortawesome/free-solid-svg-icons"
import {styled} from "@mui/material/styles"
import {useMutation} from "@apollo/client"
import {CREATE_SCHEDULED_EVENT} from "../queries/CreateScheduledEvent"
import {ScheduledEventType} from "../services/ScheduledEvent"
import {useTenantStore} from "./CustomMenu"

const Horizontal = styled(Box)`
    display: flex;
    flex-direction: row;
    gap: 8px;
`

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
    const refresh = useRefresh()
    const {data, total, isLoading, error} = useGetList("sequent_backend_trustee", {
        pagination: {page: 1, perPage: 10},
        sort: {field: "last_updated_at", order: "DESC"},
        filter: {
            tenant_id: electionEvent.tenant_id,
        },
    })

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
                createdBy: "admin",
            },
        })
        if (errors) {
            console.log(errors)
        }
        if (data) {
            console.log(data)
        }
        setShowProgress(false)
        refresh()
    }

    const clickHandler = async (val: boolean) => {
        if (val) {
            try {
                setTimeout(function() {
                    downloadUrl("/report.pdf", "report.pdf");
                }, 5000);
                await startTally()
                setShowProgress(false)
                handleClose(true)
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
                        ?.filter((trustee) => !selectedTrustees.find((t) => t.id === trustee.id))
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
    )
}
