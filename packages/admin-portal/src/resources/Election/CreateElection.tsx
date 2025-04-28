// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useState} from "react"
import {useTreeMenuData} from "@/components/menu/items/use-tree-menu-hook"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {Box, Typography, styled} from "@mui/material"
import {
    BooleanInput,
    Create,
    Identifier,
    NumberInput,
    RaRecord,
    ReferenceInput,
    SaveButton,
    SelectInput,
    SimpleForm,
    TextInput,
    Toolbar,
    useGetOne,
    useRedirect,
} from "react-admin"
import {JsonInput} from "react-admin-json-view"
import {useSearchParams} from "react-router-dom"
import {useTranslation} from "react-i18next"
import {NewResourceContext} from "@/providers/NewResourceProvider"
import {Sequent_Backend_Election_Extended} from "./ElectionDataForm"
import {addDefaultTranslationsToElement} from "@/services/i18n"
import {
    IElectionPresentation,
    ITenantSettings,
    IElectionEventPresentation,
} from "@sequentech/ui-core"
import {useMutation} from "@apollo/client"
import {CREATE_ELECTION} from "@/queries/CreateElection"
import {CreateElectionMutation} from "@/gql/graphql"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"

const Hidden = styled(Box)`
    display: none;
`

export const CreateElection: React.FC = () => {
    const {t} = useTranslation()

    const [tenantId] = useTenantStore()
    const [searchParams] = useSearchParams()
    const redirect = useRedirect()

    const [settings, setSettings] = useState<any>()
    const electionEventId = searchParams.get("electionEventId")
    const [createElection] = useMutation<CreateElectionMutation>(CREATE_ELECTION)

    const {data: electionEvent} = useGetOne("sequent_backend_election_event", {
        id: electionEventId,
    })
    const {data: tenant} = useGetOne("sequent_backend_tenant", {
        id: tenantId,
    })

    const {refetch} = useTreeMenuData(false)

    const {setLastCreatedResource} = useContext(NewResourceContext)

    const {setElectionIdFlag} = useElectionEventTallyStore()

    useEffect(() => {
        if (tenant) {
            const temp = tenant?.settings
            setSettings(temp)
        }
    }, [tenant])

    const transform = (data: Sequent_Backend_Election_Extended): RaRecord<Identifier> => {
        let i18n = addDefaultTranslationsToElement(data)
        let tenantLangConf = (tenant?.settings as ITenantSettings | undefined)?.language_conf ?? {
            enabled_language_codes: settings?.languages ?? ["en"],
            default_language_code: "en",
        }

        tenantLangConf.default_language_code = tenantLangConf.default_language_code ?? "en"
        let presentation: IElectionPresentation = {
            ...(data.presentation as IElectionPresentation),
            i18n,
            language_conf: tenantLangConf,
        }
        return {
            ...data,
            presentation,
        }
    }

    const onSubmit = async (input0: any) => {
        let electionSubmit = input0 as {
            name: string
            description?: string
            presentation: IElectionPresentation
        }
        let i18n = addDefaultTranslationsToElement(electionSubmit)
        let tenantLangConf = (tenant?.settings as ITenantSettings | undefined)?.language_conf ?? {
            enabled_language_codes: settings?.languages ?? ["en"],
            default_language_code: "en",
        }

        tenantLangConf.default_language_code = tenantLangConf.default_language_code ?? "en"
        // Set the lang conf to the same as the event, if not fallback to the tenant
        let parentLangConf =
            (electionEvent?.presentation as IElectionEventPresentation | undefined)
                ?.language_conf ?? tenantLangConf
        let presentation: IElectionPresentation = {
            ...(input0.presentation as IElectionPresentation),
            i18n,
            language_conf: parentLangConf,
        }

        electionSubmit = {
            ...electionSubmit,
            presentation,
        }

        try {
            const {data} = await createElection({
                variables: {
                    electionEventId: electionEventId,
                    name: electionSubmit.name,
                    presentation: electionSubmit.presentation,
                    description: electionSubmit.description,
                },
            })
            let id = data?.create_election?.id
            if (id) {
                refetch()
                setLastCreatedResource({id: id, type: "sequent_backend_election"})
                setElectionIdFlag(id)
                redirect(`/sequent_backend_election/${id}`)
            }
        } catch (e) {
            console.log(e)
        }
    }

    return (
        <SimpleForm
            onSubmit={onSubmit}
            toolbar={
                <Toolbar>
                    <SaveButton className="election-save-button" />
                </Toolbar>
            }
        >
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
    )
}
