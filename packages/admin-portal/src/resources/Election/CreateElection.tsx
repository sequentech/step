// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext} from "react"
import {useTreeMenuData} from "@/components/menu/items/use-tree-menu-hook"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {Box, Typography, styled} from "@mui/material"
import {
    BooleanInput,
    Create,
    NumberInput,
    ReferenceInput,
    SelectInput,
    SimpleForm,
    TextInput,
    useRedirect,
} from "react-admin"
import {JsonInput} from "react-admin-json-view"
import {useSearchParams} from "react-router-dom"
import {useTranslation} from "react-i18next"
import {NewResourceContext} from "@/providers/NewResourceProvider"

const Hidden = styled(Box)`
    display: none;
`

export const CreateElection: React.FC = () => {
    const {t} = useTranslation()

    const [tenantId] = useTenantStore()
    const [searchParams] = useSearchParams()
    const redirect = useRedirect()

    const electionEventId = searchParams.get("electionEventId")

    const {refetch} = useTreeMenuData(false)

    const {setLastCreatedResourceId} = useContext(NewResourceContext)

    return (
        <Create
            mutationOptions={{
                onSuccess: (data: any) => {
                    setLastCreatedResourceId(data.id)
                    refetch()
                    redirect(`/sequent_backend_election/${data.id}`)
                },
            }}
        >
            <SimpleForm>
                <Typography variant="h4">{t("common.resources.election")}</Typography>
                <Typography variant="body2">{t("createResource.election")}</Typography>
                <TextInput source="name" />
                <TextInput source="description" />
                <Hidden>
                    <BooleanInput source="is_consolidated_ballot_encoding" />
                    <BooleanInput source="spoil_ballot_option" />
                    <ReferenceInput source="tenant_id" reference="sequent_backend_tenant">
                        <SelectInput optionText="slug" defaultValue={tenantId} />
                    </ReferenceInput>

                    <ReferenceInput
                        source="election_event_id"
                        reference="sequent_backend_election_event"
                    >
                        <SelectInput optionText="name" defaultValue={electionEventId} />
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
                </Hidden>
            </SimpleForm>
        </Create>
    )
}
