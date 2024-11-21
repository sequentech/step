// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect} from "react"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {
    BooleanInput,
    ReferenceInput,
    SaveButton,
    Toolbar,
    SelectInput,
    SimpleForm,
    TextInput,
    RaRecord,
    useGetList,
} from "react-admin"
import {JsonInput} from "react-admin-json-view"
import {Box, CircularProgress} from "@mui/material"
import {useTranslation} from "react-i18next"
import {styled} from "@mui/material/styles"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {useCreateElectionEventStore} from "@/providers/CreateElectionEventContextProvider"
import {ElectionHeaderStyles} from "@/components/styles/ElectionHeaderStyles"

const Hidden = styled(Box)`
    display: none;
`

const ReservedSpace = styled(Box)`
    min-height: 40px;
`

interface IPullChecker<T extends RaRecord> {
    id: string
    resource: string
    dependencies: any[]
    onResolved: (result: {data: T[] | undefined; isLoading: boolean; error: any}) => void
}

const PullChecker = <T extends RaRecord>({
    id,
    resource,
    dependencies,
    onResolved,
}: IPullChecker<T>) => {
    const {globalSettings} = useContext(SettingsContext)

    const {data, isLoading, error} = useGetList<T>(
        resource,
        {filter: {id: id}},
        {
            refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
        }
    )

    useEffect(() => {
        onResolved({data, isLoading, error})
    }, [isLoading, data, error, id, ...dependencies])

    return <div />
}

export const CreateElectionEventScreen: React.FC = () => {
    const {t} = useTranslation()
    const {postDefaultValues, handleElectionCreated, handleSubmit, isLoading, newId, tenantId} =
        useCreateElectionEventStore()

    return (
        <>
            {newId && (
                <PullChecker<Sequent_Backend_Election_Event>
                    id={newId}
                    resource="sequent_backend_election_event"
                    dependencies={[isLoading, newId]}
                    onResolved={handleElectionCreated}
                />
            )}
            <SimpleForm
                defaultValues={postDefaultValues}
                onSubmit={handleSubmit}
                toolbar={
                    <Toolbar>
                        <SaveButton className="election-event-save-button" disabled={isLoading} />
                    </Toolbar>
                }
            >
                <ElectionHeaderStyles.Title>
                    {t("createResource.electionEvent")}
                </ElectionHeaderStyles.Title>

                <TextInput source="name" />
                <TextInput source="description" />
                <Hidden>
                    <SelectInput
                        source="encryption_protocol"
                        choices={[{id: "RSA256", name: "RSA256"}]}
                        defaultValue={"RSA256"}
                    />
                    <ReferenceInput source="tenant_id" reference="sequent_backend_tenant">
                        <SelectInput optionText="slug" defaultValue={tenantId} />
                    </ReferenceInput>
                    <BooleanInput source="is_archived" defaultValue={false} />
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
                </Hidden>
                <ReservedSpace>{isLoading ? <CircularProgress /> : null}</ReservedSpace>
            </SimpleForm>
            <hr />
        </>
    )
}
