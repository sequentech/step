// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Button, CircularProgress, Menu, MenuItem, Typography} from "@mui/material"
import React, {useState} from "react"
import {
    BooleanInput,
    Edit,
    ReferenceManyField,
    SelectInput,
    SimpleForm,
    TextField,
    TextInput,
    useRecordContext,
    useRefresh,
} from "react-admin"
import {JsonInput} from "react-admin-json-view"
import {ElectionEventList} from "./ElectionEventList"
import {HorizontalBox} from "../../components/HorizontalBox"
import {ChipList} from "../../components/ChipList"
import {Link} from "react-router-dom"
import {CreateScheduledEventMutation, Sequent_Backend_Election_Event} from "../../gql/graphql"
import {IconButton, isUndefined} from "@sequentech/ui-essentials"
import {faPieChart, faPlusCircle} from "@fortawesome/free-solid-svg-icons"
import {ScheduledEventType} from "../../services/ScheduledEvent"
import {useTenantStore} from "../../components/CustomMenu"
import {CREATE_SCHEDULED_EVENT} from "../../queries/CreateScheduledEvent"
import {useMutation} from "@apollo/client"

const ElectionEventListForm: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const [showMenu, setShowMenu] = useState(false)
    const [anchorEl, setAnchorEl] = React.useState<null | HTMLElement>(null)
    const [showProgress, setShowProgress] = useState(false)
    const [tenantId] = useTenantStore()
    const [createScheduledEvent] = useMutation<CreateScheduledEventMutation>(CREATE_SCHEDULED_EVENT)
    const refresh = useRefresh()

    const handleActionsButtonClick: React.MouseEventHandler<HTMLButtonElement> = (event) => {
        setAnchorEl(event.currentTarget)
        setShowMenu(true)
    }

    const createBulletinBoardAction = async () => {
        setShowMenu(false)
        setShowProgress(true)

        const {data, errors} = await createScheduledEvent({
            variables: {
                tenantId: tenantId,
                electionEventId: record.id,
                eventProcessor: ScheduledEventType.CREATE_BOARD,
                cronConfig: undefined,
                eventPayload: {
                    board_name: record.id.replaceAll('-', ''),
                    trustee_pks: [
                        "1uPVrX1ZRmSfRsw8DIGCLzpV4sZYBHEh0zJojLHffrA",
                        "NSEzWRG05+35Fo8b7n1Ci+WtP9uKgGJ1+DxLtXKQdS4"
                    ],
                    threshold: 2
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

    const createKeysAction = async () => {
        setShowMenu(false)
        setShowProgress(true)

        const {data, errors} = await createScheduledEvent({
            variables: {
                tenantId: tenantId,
                electionEventId: record.id,
                eventProcessor: ScheduledEventType.CREATE_KEYS,
                cronConfig: undefined,
                eventPayload: {
                    board_name: record.id.replaceAll('-', ''),
                    trustee_pks: [
                        "1uPVrX1ZRmSfRsw8DIGCLzpV4sZYBHEh0zJojLHffrA",
                        "xDO0LdOBRejUpXZ+EFWka1Q9jxbkqJLYea4jkRHAQMw"
                    ],
                    threshold: 2
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

    return (
        <SimpleForm>
            <Link to={`/sequent_backend_election_event/${record.id}/show`}>
                <Button>
                    <IconButton icon={faPieChart} fontSize="24px" />
                    Show Dashboard
                </Button>
            </Link>
            <Typography variant="h4">Election Event</Typography>
            <Typography variant="body2">Election event configuration</Typography>
            <Button onClick={handleActionsButtonClick}>
                Actions {showProgress ? <CircularProgress /> : null}
            </Button>
            <Menu
                id="election-event-actions-menu"
                anchorEl={anchorEl}
                open={showMenu}
                onClose={() => setShowMenu(false)}
            >
                <MenuItem
                    onClick={createBulletinBoardAction}
                    disabled={!!record.bulletin_board_reference}
                >
                    Create Bulletin Board
                </MenuItem>
                <MenuItem
                    onClick={createKeysAction}
                >
                    Create Keys
                </MenuItem>
            </Menu>
            <Typography variant="h5">ID</Typography>
            <TextField source="id" />
            <TextInput source="name" />
            <TextInput source="description" />
            <SelectInput source="encryption_protocol" choices={[{id: "RSA256", name: "RSA256"}]} />
            <BooleanInput source="is_archived" />
            <BooleanInput source="is_audit" />
            <Typography variant="h5">Elections</Typography>
            <ReferenceManyField
                label="Elections"
                reference="sequent_backend_election"
                target="election_event_id"
            >
                <HorizontalBox>
                    <ChipList
                        source="sequent_backend_election"
                        filterFields={["election_event_id"]}
                    />
                </HorizontalBox>
            </ReferenceManyField>
            <Link
                to={{
                    pathname: "/sequent_backend_election/create",
                }}
                state={{
                    record: {
                        election_event_id: record.id,
                        tenant_id: record.tenant_id,
                    },
                }}
            >
                <Button>
                    <IconButton icon={faPlusCircle} fontSize="24px" />
                    Add election
                </Button>
            </Link>
            <Typography variant="h5">Areas</Typography>
            <ReferenceManyField
                label="Areas"
                reference="sequent_backend_area"
                target="election_event_id"
            >
                <ChipList source="sequent_backend_area" filterFields={["election_event_id"]} />
            </ReferenceManyField>
            <Link
                to={{
                    pathname: "/sequent_backend_area/create",
                }}
                state={{
                    record: {
                        election_event_id: record.id,
                        tenant_id: record.tenant_id,
                    },
                }}
            >
                <Button>
                    <IconButton icon={faPlusCircle} fontSize="24px" />
                    Add area
                </Button>
            </Link>
            <JsonInput
                source="bulletin_board_reference"
                jsonString={false}
                reactJsonOptions={{
                    name: null,
                    collapsed: true,
                    enableClipboard: true,
                    displayDataTypes: false,
                }}
            />
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
                source="voting_channels"
                jsonString={false}
                reactJsonOptions={{
                    name: null,
                    collapsed: true,
                    enableClipboard: true,
                    displayDataTypes: false,
                }}
            />
            <JsonInput
                source="voting_channels"
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
            <TextInput source="user_boards" />
            <TextInput source="audit_election_event_id" />
            <Typography variant="h5">Documents</Typography>
            <ReferenceManyField
                label="Documents"
                reference="sequent_backend_document"
                target="election_event_id"
            >
                <HorizontalBox>
                    <ChipList
                        source="sequent_backend_document"
                        filterFields={["election_event_id"]}
                    />
                </HorizontalBox>
            </ReferenceManyField>
        </SimpleForm>
    )
}

export const EditElectionList: React.FC = () => {
    return (
        <ElectionEventList
            aside={
                <Edit sx={{flexGrow: 2, width: "50%", flexShrink: 0}}>
                    <ElectionEventListForm />
                </Edit>
            }
        />
    )
}
