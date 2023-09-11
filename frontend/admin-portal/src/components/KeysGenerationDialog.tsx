// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {Dialog, isNumber} from "@sequentech/ui-essentials"
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
    Sequent_Backend_Election_Event,
    Sequent_Backend_Trustee,
    CreateScheduledEventMutation,
} from "../gql/graphql"
import {useGetList, useRefresh} from "react-admin"
import {StyledChip} from "./StyledChip"
import {IconButton} from "@sequentech/ui-essentials"
import {faPlusCircle} from "@fortawesome/free-solid-svg-icons"
import {styled} from "@mui/material/styles"
import {useMutation} from "@apollo/client"
import {CREATE_SCHEDULED_EVENT} from "../queries/CreateScheduledEvent"
import {ScheduledEventType} from "../services/ScheduledEvent"

const Horizontal = styled(Box)`
    display: flex;
    flex-direction: row;
    gap: 8px;
`

export interface KeysGenerationDialogProps {
    show: boolean
    handleClose: (val: boolean) => void
    electionEvent: Sequent_Backend_Election_Event
}

export const KeysGenerationDialog: React.FC<KeysGenerationDialogProps> = ({
    show,
    handleClose,
    electionEvent,
}) => {
    const [selectedTrustees, setSelectedTrustees] = useState<Array<Sequent_Backend_Trustee>>([])
    const [createScheduledEvent] = useMutation<CreateScheduledEventMutation>(CREATE_SCHEDULED_EVENT)
    const [showProgress, setShowProgress] = useState(false)
    const [threshold, setThreshold] = useState(2)
    const [trustee, setTrustee] = useState<Sequent_Backend_Trustee | null>(null)
    const refresh = useRefresh()
    const {data, total, isLoading, error} = useGetList("sequent_backend_trustee", {
        pagination: {page: 1, perPage: 10},
        sort: {field: "last_updated_at", order: "DESC"},
        filter: {
            is_protocol_manager: false,
            tenant_id: electionEvent.tenant_id,
        },
    })

    if (isLoading || error) {
        return null
    }

    const generateKeys = async () => {
        setShowProgress(true)

        const {data, errors} = await createScheduledEvent({
            variables: {
                tenantId: electionEvent.tenant_id,
                electionEventId: electionEvent.id,
                eventProcessor: ScheduledEventType.CREATE_KEYS,
                cronConfig: undefined,
                eventPayload: {
                    trustee_pks: selectedTrustees.map((trustee) => trustee.public_key),
                    threshold: threshold,
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
                await generateKeys()
                handleClose(true)
            } catch (error) {
                console.log(`Error trying to create keys: ${error}`)
                handleClose(false)
            }
        } else {
            handleClose(false)
        }
    }

    const handleTrusteeChange = (event: SelectChangeEvent<Sequent_Backend_Trustee | null>) => {
        let id = event.target.value
        let trustee: Sequent_Backend_Trustee | undefined = (data as Array<Sequent_Backend_Trustee> | undefined)?.find(t => t.id === id)
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

    const isValidThreshold = (value: number): boolean => {
        return !isNaN(value) && value > 2 && value <= selectedTrustees.length
    }

    const handleThresholdChange: React.ChangeEventHandler<
        HTMLInputElement | HTMLTextAreaElement
    > = (event) => {
        let value = Number(event.target.value)
        setThreshold(value)
    }

    return (
        <Dialog
            handleClose={clickHandler}
            open={show}
            title="Key Generation Dialog"
            ok="OK"
            cancel="Cancel"
            variant="info"
        >
            <Typography variant="body1">Generate Keys for Event</Typography>
            <TextField
                value={threshold}
                error={!isValidThreshold(threshold)}
                type="number"
                InputLabelProps={{
                    shrink: true,
                }}
                variant="filled"
                onChange={handleThresholdChange}
            />
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
                    renderValue={value => value?.name}
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
            {showProgress ? <CircularProgress /> : null}
        </Dialog>
    )
}
