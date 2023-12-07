// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext} from "react"
import {useTreeMenuData} from "@/components/menu/items/use-tree-menu-hook"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {Box, Typography, styled} from "@mui/material"
import {
    BooleanInput,
    SimpleForm,
    TextInput,
    NumberInput,
    SelectInput,
    ReferenceInput,
    Create,
    useRedirect,
} from "react-admin"
import {JsonInput} from "react-admin-json-view"
import {useSearchParams} from "react-router-dom"
import {useTranslation} from "react-i18next"
import {NewResourceContext} from "@/providers/NewResourceProvider"

const Hidden = styled(Box)`
    display: none;
`

export const CreateContest: React.FC = () => {
    const {t} = useTranslation()

    const [tenantId] = useTenantStore()
    const [searchParams] = useSearchParams()
    const redirect = useRedirect()

    const electionEventId = searchParams.get("electionEventId")
    const electionId = searchParams.get("electionId")

    const {refetch} = useTreeMenuData(false)
    const {setLastCreatedResource} = useContext(NewResourceContext)

    return (
        <Create
            mutationOptions={{
                onSuccess: (data: any) => {
                    refetch()
                    setLastCreatedResource({id: data.id, type: "sequent_backend_contest"})
                    redirect(`/sequent_backend_contest/${data.id}`)
                },
            }}
        >
            <SimpleForm>
                <Typography variant="h4">{t("common.resources.contest")}</Typography>
                <Typography variant="body2">{t("createResource.contest")}</Typography>
                <TextInput source="name" />
                <TextInput source="description" />

                <Hidden>
                    <BooleanInput source="is_acclaimed" />
                    <BooleanInput source="is_active" />
                    <NumberInput source="min_votes" />
                    <NumberInput source="max_votes" />
                    <SelectInput
                        source="voting_type"
                        choices={[{id: "first-past-the-post", name: "First Past The Post"}]}
                    />
                    <SelectInput
                        source="counting_algorithm"
                        choices={[{id: "plurality-at-large", name: "Plurality At Large"}]}
                    />
                    <BooleanInput source="is_encrypted" />
                    <ReferenceInput source="tenant_id" reference="sequent_backend_tenant">
                        <SelectInput optionText="slug" defaultValue={tenantId} />
                    </ReferenceInput>

                    <ReferenceInput
                        source="election_event_id"
                        reference="sequent_backend_election_event"
                    >
                        <SelectInput optionText="name" defaultValue={electionEventId} />
                    </ReferenceInput>

                    <ReferenceInput source="election_id" reference="sequent_backend_election">
                        <SelectInput optionText="name" defaultValue={electionId} />
                    </ReferenceInput>

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
                        source="tally_configuration"
                        jsonString={false}
                        reactJsonOptions={{
                            name: null,
                            collapsed: true,
                            enableClipboard: true,
                            displayDataTypes: false,
                        }}
                    />
                    <JsonInput
                        source="conditions"
                        jsonString={false}
                        reactJsonOptions={{
                            name: null,
                            collapsed: true,
                            enableClipboard: true,
                            displayDataTypes: false,
                        }}
                    />
                </Hidden>
            </SimpleForm>
        </Create>
    )
}
