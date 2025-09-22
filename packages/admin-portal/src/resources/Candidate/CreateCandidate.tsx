// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useTenantStore} from "@/providers/TenantContextProvider"
import {Box, Typography, styled} from "@mui/material"
import React, {useContext} from "react"
import {
    BooleanInput,
    SimpleForm,
    TextInput,
    SelectInput,
    ReferenceInput,
    Create,
    useRedirect,
    Toolbar,
    SaveButton,
    RaRecord,
    Identifier,
} from "react-admin"
import {JsonInput} from "react-admin-json-view"
import {useSearchParams} from "react-router-dom"
import {useTreeMenuData} from "@/components/menu/items/use-tree-menu-hook"
import {useTranslation} from "react-i18next"
import {NewResourceContext} from "@/providers/NewResourceProvider"
import {Sequent_Backend_Candidate_Extended} from "./CandidateDataForm"
import {addDefaultTranslationsToElement} from "@/services/i18n"
import {ICandidatePresentation} from "@sequentech/ui-core"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"

const Hidden = styled(Box)`
    display: none;
`

export const CreateCandidate: React.FC = () => {
    const {t} = useTranslation()

    const [tenantId] = useTenantStore()
    const [searchParams] = useSearchParams()
    const redirect = useRedirect()

    const electionEventId = searchParams.get("electionEventId")
    const contestId = searchParams.get("contestId")
    const {setCandidateIdFlag, setContestIdFlag} = useElectionEventTallyStore()

    const {setLastCreatedResource} = useContext(NewResourceContext)
    const {refetch} = useTreeMenuData(false)

    const transform = (data: Sequent_Backend_Candidate_Extended): RaRecord<Identifier> => {
        let i18n = addDefaultTranslationsToElement(data)
        let presentation: ICandidatePresentation = {
            ...(data.presentation as ICandidatePresentation),
            i18n,
        }
        return {
            ...data,
            presentation,
        }
    }

    return (
        <Create
            mutationOptions={{
                onSuccess: (data: Sequent_Backend_Candidate_Extended) => {
                    refetch()
                    setLastCreatedResource({id: data.id, type: "sequent_backend_candidate"})
                    setCandidateIdFlag(data.id)
                    redirect(`/sequent_backend_candidate/${data.id}`)
                },
            }}
            transform={transform}
        >
            <SimpleForm
                toolbar={
                    <Toolbar>
                        <SaveButton className="candidate-save-button" />
                    </Toolbar>
                }
            >
                <Typography variant="h4">{t("common.resources.candidate")}</Typography>
                <Typography variant="body2">{t("createResource.candidate")}</Typography>
                <TextInput source="name" />
                <TextInput source="description" />
                <Hidden>
                    <TextInput source="type" />
                    <BooleanInput source="is_public" />
                    <ReferenceInput source="tenant_id" reference="sequent_backend_tenant">
                        <SelectInput optionText="slug" defaultValue={tenantId} />
                    </ReferenceInput>

                    <ReferenceInput
                        source="election_event_id"
                        reference="sequent_backend_election_event"
                    >
                        <SelectInput optionText="name" defaultValue={electionEventId} />
                    </ReferenceInput>

                    <ReferenceInput source="contest_id" reference="sequent_backend_contest">
                        <SelectInput optionText="name" defaultValue={contestId} />
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
                </Hidden>
            </SimpleForm>
        </Create>
    )
}
