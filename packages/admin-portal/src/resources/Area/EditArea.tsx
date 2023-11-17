// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Button, CircularProgress, Menu, MenuItem, Typography} from "@mui/material"
import React, {useState} from "react"
import {
    Edit,
    EditBase,
    Identifier,
    ReferenceField,
    ReferenceManyField,
    SimpleForm,
    TextField,
    TextInput,
    useRecordContext,
    useRedirect,
    useRefresh,
} from "react-admin"
import {ListArea} from "./ListArea"
import {JsonInput} from "react-admin-json-view"
import {ChipList} from "../../components/ChipList"
import {CreateScheduledEventMutation, Sequent_Backend_Area} from "../../gql/graphql"
import {Link} from "react-router-dom"
import {IconButton} from "@sequentech/ui-essentials"
import {faPlusCircle} from "@fortawesome/free-solid-svg-icons"
import {useTenantStore} from "../../components/CustomMenu"
import {useMutation} from "@apollo/client"
import {CREATE_SCHEDULED_EVENT} from "../../queries/CreateScheduledEvent"
import {ScheduledEventType} from "../../services/ScheduledEvent"

interface EditAreaProps {
    id?: Identifier | undefined
    close?: () => void
}

export const EditArea: React.FC<EditAreaProps> = (props) => {
    const {id, close} = props
    const refresh = useRefresh()

    const onSuccess = async (res: any) => {
        refresh()
        if (close) {
            close()
        }
    }

    return (
        <Edit
            id={id}
            resource="sequent_backend_area"
            mutationMode="pessimistic"
            mutationOptions={{onSuccess}}
            redirect={false}
        >
            <SimpleForm>
                <Typography variant="h4">Area</Typography>
                <Typography variant="body2">Area configuration</Typography>

                <TextInput source="name" />

                {/* <Button onClick={handleActionsButtonClick}>
                Actions {showProgress ? <CircularProgress /> : null}
            </Button>
            <Menu
                id="election-event-actions-menu"
                anchorEl={anchorEl}
                open={showMenu}
                onClose={() => setShowMenu(false)}
            ></Menu>
            <Typography variant="h5">ID</Typography>
            <TextField source="id" /> */}

                {/* <TextInput source="description" />
            <TextInput source="type" />
            <Typography variant="h5">Election Event</Typography>

                <TextField source="name" />
            </ReferenceField>
            <ReferenceManyField
                label="Area Contests"
                reference="sequent_backend_area_contest"
                target="area_id"
            >
                <ChipList
                    source="sequent_backend_area_contest"
                    filterFields={["election_event_id", "area_id"]}
                />
            </ReferenceManyField>
            <Link
                to={{
                    pathname: "/sequent_backend_area_contest/create",
                }}
                state={{
                    record: {
                        area_id: record.id,
                        election_event_id: record.election_event_id,
                        tenant_id: record.tenant_id,
                    },
                }}
            >
                <Button>
                    <IconButton icon={faPlusCircle} fontSize="24px" />
                    Add area contest
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
            /> */}
            </SimpleForm>
        </Edit>
    )
}
function refresh() {
    throw new Error("Function not implemented.")
}
