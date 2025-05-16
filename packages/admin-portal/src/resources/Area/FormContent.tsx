// SPDX-FileCopyrightText: 2025 Enric Badia <enric@xtremis.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useState} from "react"
import {UpsertAreaMutation} from "@/gql/graphql"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {GET_AREAS_EXTENDED} from "@/queries/GetAreasExtended"
import {UPSERT_AREA} from "@/queries/UpsertArea"
import {useMutation, useQuery} from "@apollo/client"
import {
    useRefresh,
    useNotify,
    RecordContext,
    SimpleForm,
    Toolbar,
    SaveButton,
    TextInput,
    AutocompleteArrayInput,
    ReferenceArrayInput,
} from "react-admin"
import {useTranslation} from "react-i18next"
import {UpsertAreaProps} from "./UpsertArea"
import SelectArea from "@/components/area/SelectArea"
import {PageHeaderStyles} from "@/components/styles/PageHeaderStyles"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"

/**
 * FormContent component for creating or updating an Area entity.
 *
 * This component renders a form for area details, including name, description, contests, parent area, and hidden fields for election event and tenant.
 * It handles form submission using a GraphQL mutation, provides notifications on success or error, and refreshes the data.
 *
 * @param props - The properties for the form, including:
 *   - record: The current area record (if editing).
 *   - id: The area ID (if editing).
 *   - electionEventId: The election event ID associated with the area.
 *   - close: Optional callback to close the form/modal.
 *
 * @returns A React element rendering the area form.
 *
 * @remarks
 * - Uses react-admin components for form rendering and submission.
 * - Integrates with GraphQL for data fetching and mutations.
 * - Supports contest selection with autocomplete and filtering.
 */
export const FormContent: React.FC<UpsertAreaProps> = (props) => {
    const {record, id, electionEventId, close} = props

    const refresh = useRefresh()
    const notify = useNotify()
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const aliasRenderer = useAliasRenderer()

    const contestFilterToQuery = (searchText: string) => {
        if (!searchText || searchText.length == 0) {
            return {name: ""}
        }
        return {"name@_ilike,alias@_ilike": searchText.trim()}
    }

    const [upsertArea] = useMutation<UpsertAreaMutation>(UPSERT_AREA, {
        refetchQueries: [
            {
                query: GET_AREAS_EXTENDED,
                variables: {
                    electionEventId,
                    areaId: id,
                },
            },
        ],
    })

    const {data: areas} = useQuery(GET_AREAS_EXTENDED, {
        variables: {
            electionEventId,
            areaId: id,
        },
    })
    const parseValues = (incoming: any) => {
        const temp = {...incoming}

        temp.area_contest_ids = areas?.sequent_backend_area_contest?.map(
            (area: any) => area.contest.id
        )

        return temp
    }

    const onSubmit = async (values: any) => {

        try {
            const {data} = await upsertArea({
                variables: {
                    id: values.id,
                    name: values.name,
                    description: values.description,
                    presentation: values.presentation,
                    tenantId: tenantId,
                    electionEventId,
                    parentId: values.parent_id,
                    areaContestsIds: values.area_contest_ids,
                    annotations: values.annotations,
                    labels: values.labels,
                    type: values.type,
                },
            })
            refresh()
            notify(t("areas.createAreaSuccess"), {type: "success"})
            if (close) {
                close()
            }
        } catch (e) {
            console.log("aa error creating", e)
            refresh()
            notify("areas.createAreaError", {type: "error"})
            if (close) {
                close()
            }
        }
    }
    return (
        <RecordContext.Consumer>
            {(incoming) => {
                const parsedValue = parseValues(incoming)
                return (
                    <SimpleForm
                        record={parsedValue}
                        onSubmit={onSubmit}
                        toolbar={
                            <Toolbar>
                                <SaveButton />
                            </Toolbar>
                        }
                    >
                        <PageHeaderStyles.Title>{t("areas.common.title")}</PageHeaderStyles.Title>
                        <PageHeaderStyles.SubTitle>
                            {t("areas.common.subTitle")}
                        </PageHeaderStyles.SubTitle>

                        <TextInput source="name" />
                        <TextInput source="description" />

                        <ReferenceArrayInput
                            label={t("areas.sequent_backend_area_contest")}
                            reference="sequent_backend_contest"
                            source="area_contest_ids"
                            filter={{
                                tenant_id: tenantId,
                                election_event_id: electionEventId,
                            }}
                            perPage={100} // // Setting initial larger records size
                            enableGetChoices={({q}) => q && q.length >= 3}
                        >
                            <AutocompleteArrayInput
                                className="area-contest"
                                fullWidth={true}
                                optionText={aliasRenderer}
                                filterToQuery={contestFilterToQuery}
                                debounce={100}
                            />
                        </ReferenceArrayInput>
                        <SelectArea
                            tenantId={tenantId}
                            electionEventId={electionEventId}
                            source="parent_id"
                        />
                        {record ? (
                            <>
                                <TextInput
                                    label="Election Event"
                                    source="election_event_id"
                                    defaultValue={record?.id || ""}
                                    style={{display: "none"}}
                                />
                                <TextInput
                                    label="Tenant"
                                    source="tenant_id"
                                    defaultValue={record?.tenant_id || ""}
                                    style={{display: "none"}}
                                />
                            </>
                        ) : null}
                    </SimpleForm>
                )
            }}
        </RecordContext.Consumer>
    )
}
