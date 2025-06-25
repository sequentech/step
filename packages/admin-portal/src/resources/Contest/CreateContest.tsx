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
    Toolbar,
    SaveButton,
    Identifier,
    RaRecord,
} from "react-admin"
import {JsonInput} from "react-admin-json-view"
import {useSearchParams} from "react-router-dom"
import {useTranslation} from "react-i18next"
import {NewResourceContext} from "@/providers/NewResourceProvider"
import {ICountingAlgorithm, IVotingType} from "./constants"
import {CandidatesOrder, IContestPresentation} from "@sequentech/ui-core"
import {Sequent_Backend_Contest_Extended} from "./EditContestDataForm"
import {addDefaultTranslationsToElement} from "@/services/i18n"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"

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

    const votingTypesChoices = () => {
        return (Object.values(IVotingType) as IVotingType[]).map((value) => ({
            id: value,
            name: t(`contestScreen.options.${value.toLowerCase()}`),
        }))
    }

    const {setContestIdFlag} = useElectionEventTallyStore()

    const transform = (data: Sequent_Backend_Contest_Extended): RaRecord<Identifier> => {
        let i18n = addDefaultTranslationsToElement(data)
        let presentation: IContestPresentation = {
            ...(data.presentation as IContestPresentation),
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
                onSuccess: (data: any) => {
                    refetch()
                    setLastCreatedResource({id: data.id, type: "sequent_backend_contest"})
                    setContestIdFlag(data.id)
                    redirect(`/sequent_backend_contest/${data.id}`)
                },
            }}
            transform={transform}
        >
            <SimpleForm
                toolbar={
                    <Toolbar>
                        <SaveButton className="contest-save-button" />
                    </Toolbar>
                }
            >
                <Typography variant="h4">{t("common.resources.contest")}</Typography>
                <Typography variant="body2">{t("createResource.contest")}</Typography>
                <TextInput source="name" />
                <TextInput source="description" />

                <Hidden>
                    <BooleanInput source="is_acclaimed" />
                    <BooleanInput source="is_active" defaultValue={true} />
                    <NumberInput source="min_votes" defaultValue="0" />
                    <NumberInput source="max_votes" defaultValue="1" />
                    <NumberInput source="winning_candidates_num" defaultValue={1} />
                    <SelectInput
                        source="voting_type"
                        defaultValue={IVotingType.NON_PREFERENTIAL}
                        choices={votingTypesChoices()}
                    />
                    <SelectInput
                        source="counting_algorithm"
                        defaultValue={ICountingAlgorithm.PLURALITY_AT_LARGE}
                        choices={[
                            {
                                id: ICountingAlgorithm.PLURALITY_AT_LARGE,
                                name: t("contestScreen.options.plurality-at-large"),
                            },
                        ]}
                    />
                    <SelectInput
                        source="presentation.candidates_order"
                        defaultValue={CandidatesOrder.ALPHABETICAL}
                        choices={[
                            {
                                id: CandidatesOrder.ALPHABETICAL,
                                name: CandidatesOrder.ALPHABETICAL,
                            },
                        ]}
                    />
                    <BooleanInput source="is_encrypted" defaultValue={true} />
                    <TextInput source="presentation.candidates_order" />
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
