// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box, Button, MenuItem, Menu, Typography} from "@mui/material"
import React, {useState} from "react"
import {
    BooleanInput,
    Edit,
    NumberInput,
    ReferenceField,
    ReferenceManyField,
    SimpleForm,
    TextField,
    TextInput,
    useRecordContext,
} from "react-admin"
import {HorizontalBox} from "../../components/HorizontalBox"
import {ListElection} from "./ListElection"
import {ChipList} from "../../components/ChipList"
import {JsonInput} from "react-admin-json-view"
import {Link} from "react-router-dom"
import {IconButton} from "@sequentech/ui-essentials"
import {CreateScheduledEventMutation, Sequent_Backend_Election} from "../../gql/graphql"
import {faPlusCircle} from "@fortawesome/free-solid-svg-icons"
import {useMutation} from "@apollo/client"
import {getVotingStatus} from "../../services/ElectionStatus"
import {CircularProgress} from "@mui/material"
import {useRefresh} from "react-admin"
import {CREATE_SCHEDULED_EVENT} from "../../queries/CreateScheduledEvent"
import {ScheduledEventType} from "../../services/ScheduledEvent"
import {useTenantStore} from "../../providers/TenantContextProvider"
import {EVotingStatus} from "@sequentech/ui-core"

const ElectionForm: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election>()
    const refresh = useRefresh()
    const [showMenu, setShowMenu] = useState(false)
    const [anchorEl, setAnchorEl] = React.useState<null | HTMLElement>(null)
    const [createScheduledEvent] = useMutation<CreateScheduledEventMutation>(CREATE_SCHEDULED_EVENT)
    const [tenantId] = useTenantStore()
    const [showProgress, setShowProgress] = useState(false)

    const handleActionsButtonClick: React.MouseEventHandler<HTMLButtonElement> = (event) => {
        setAnchorEl(event.currentTarget)
        setShowMenu(true)
    }

    const changeVotingStatusAction = async (nextStatus: EVotingStatus) => {
        setShowMenu(false)
        setShowProgress(true)

        const {data, errors} = await createScheduledEvent({
            variables: {
                tenantId: tenantId,
                electionEventId: record?.election_event_id,
                eventProcessor: ScheduledEventType.UPDATE_VOTING_STATUS,
                cronConfig: undefined,
                eventPayload: {
                    election_id: record?.id,
                    status: nextStatus,
                },
                createdBy: "admin",
            },
        })
        setShowProgress(false)
        refresh()
    }

    let votingStatus = getVotingStatus(record?.status)

    return (
        <Box sx={{flexGrow: 2, flexShrink: 0}}>
            <SimpleForm>
                <Typography variant="h4">Election</Typography>
                <Button onClick={handleActionsButtonClick}>
                    Actions {showProgress ? <CircularProgress /> : null}
                </Button>
                <Menu
                    id="election-actions-menu"
                    anchorEl={anchorEl}
                    open={showMenu}
                    onClose={() => setShowMenu(false)}
                >
                    <MenuItem
                        onClick={() => changeVotingStatusAction(EVotingStatus.OPEN)}
                        disabled={EVotingStatus.OPEN === votingStatus}
                    >
                        Open Voting
                    </MenuItem>
                    <MenuItem
                        onClick={() => changeVotingStatusAction(EVotingStatus.PAUSED)}
                        disabled={EVotingStatus.OPEN !== votingStatus}
                    >
                        Pause Voting
                    </MenuItem>
                    <MenuItem
                        onClick={() => changeVotingStatusAction(EVotingStatus.CLOSED)}
                        disabled={EVotingStatus.CLOSED === votingStatus}
                    >
                        Close Voting
                    </MenuItem>
                </Menu>
                <Typography variant="body2">Election configuration</Typography>
                <Typography variant="h5">ID</Typography>
                <TextField source="id" />
                <TextInput source="name" />
                <TextInput source="description" />
                <BooleanInput source="is_consolidated_ballot_encoding" />
                <BooleanInput source="spoil_ballot_option" />
                <Typography variant="h5">Election Event</Typography>
                <ReferenceField
                    label="Election Event"
                    reference="sequent_backend_election_event"
                    source="election_event_id"
                >
                    <TextField source="name" />
                </ReferenceField>
                <Typography variant="h5">Contests</Typography>
                <ReferenceManyField
                    label="Contests"
                    reference="sequent_backend_contest"
                    target="election_id"
                >
                    <HorizontalBox>
                        <ChipList
                            source="sequent_backend_contest"
                            filterFields={["election_event_id", "election_id"]}
                        />
                    </HorizontalBox>
                </ReferenceManyField>
                <Link
                    to={{
                        pathname: "/sequent_backend_contest/create",
                    }}
                    state={{
                        record: {
                            election_id: record?.id,
                            election_event_id: record?.election_event_id,
                            tenant_id: record?.tenant_id,
                        },
                    }}
                >
                    <Button>
                        <IconButton icon={faPlusCircle as any} fontSize="24px" />
                        Add contest
                    </Button>
                </Link>
                <JsonInput
                    source="labels"
                    jsonString={false}
                    reactJsonOptions={{
                        name: null,
                        collapsed: true,
                        enableClipboard: true,
                        displayDataTypes: false,
                    }}
                />
                <JsonInput
                    source="annotations"
                    jsonString={false}
                    reactJsonOptions={{
                        name: null,
                        collapsed: true,
                        enableClipboard: true,
                        displayDataTypes: false,
                    }}
                />
                <JsonInput
                    source="presentation"
                    jsonString={false}
                    reactJsonOptions={{
                        name: null,
                        collapsed: true,
                        enableClipboard: true,
                        displayDataTypes: false,
                    }}
                />
                <JsonInput
                    source="dates"
                    jsonString={false}
                    reactJsonOptions={{
                        name: null,
                        collapsed: true,
                        enableClipboard: true,
                        displayDataTypes: false,
                    }}
                />
                <JsonInput
                    source="status"
                    jsonString={false}
                    reactJsonOptions={{
                        name: null,
                        collapsed: true,
                        enableClipboard: true,
                        displayDataTypes: false,
                    }}
                />
                <TextInput source="eml" />
                <NumberInput source="num_allowed_revotes" />
            </SimpleForm>
        </Box>
    )
}

export const EditElection: React.FC = (props) => (
    <ListElection
        aside={
            <Edit sx={{flexGrow: 2, width: "50%", flexShrink: 0}}>
                <ElectionForm />
            </Edit>
        }
    />
)
