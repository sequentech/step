// SPDX-FileCopyrightText: 2025 Enric Badia <enric@xtremis.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

import SelectArea from "@/components/area/SelectArea"
import {TextInput, ReferenceArrayInput, AutocompleteArrayInput, Identifier} from "react-admin"
import {PageHeaderStyles} from "../../components/styles/PageHeaderStyles"
import {useTranslation} from "react-i18next"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import {useTenantStore} from "@/providers/TenantContextProvider"
import React from "react"

interface AreaFormProps {
    electionEventId: Identifier | undefined
}

export const AreaForm: React.FC<AreaFormProps> = (props) => {
    const {electionEventId} = props

    const {t} = useTranslation()

    const [tenantId] = useTenantStore()
    const aliasRenderer = useAliasRenderer()

    const contestFilterToQuery = (searchText: string) => {
        if (!searchText || searchText.length == 0) {
            return {name: ""}
        }
        return {"name@_ilike,alias@_ilike": searchText.trim()}
    }

    return (
        <>
            <PageHeaderStyles.Title>{t("areas.common.title")}</PageHeaderStyles.Title>
            <PageHeaderStyles.SubTitle>{t("areas.common.subTitle")}</PageHeaderStyles.SubTitle>

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
            <SelectArea tenantId={tenantId} electionEventId={electionEventId} source="parent_id" />
        </>
    )
}
