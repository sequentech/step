// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect} from "react"
import {
    SimpleForm,
    TextInput,
    Create,
    useRecordContext,
    ReferenceInput,
    SelectInput,
    Identifier,
} from "react-admin"
import {PageHeaderStyles} from "../../components/styles/PageHeaderStyles"
import {useTranslation} from "react-i18next"
import {
    Sequent_Backend_Area,
    Sequent_Backend_Contest,
    Sequent_Backend_Tally_Sheet,
} from "@/gql/graphql"
import {useQuery} from "@apollo/client"
import {GET_CONTESTS_EXTENDED} from "@/queries/GetContestsExtended"
import {FormControl, InputLabel, MenuItem, Select, SelectChangeEvent} from "@mui/material"

type CreateTallySheetProps = {
    contest: Sequent_Backend_Contest
    doSelectArea: (areaId: Identifier) => void
}

export const CreateTallySheet: React.FC<CreateTallySheetProps> = (props) => {
    const {contest, doSelectArea} = props
    const {t} = useTranslation()

    const [areasList, setAreasList] = React.useState<Sequent_Backend_Area[]>([])
    const [area, setArea] = React.useState<string | null>(null)

    const {data: areas} = useQuery(GET_CONTESTS_EXTENDED, {
        variables: {
            electionEventId: contest.election_event_id,
            contestId: contest.id,
            tenantId: contest.tenant_id,
        },
    })

    useEffect(() => {
        if (areas) {
            console.log("areas", areas)
            const areatListTemp = areas.sequent_backend_area_contest.map(
                (item: {area: {id: string; name: string}}) => {
                    return {
                        id: item.area.id,
                        name: item.area.name,
                    }
                }
            )
            console.log("areas", areatListTemp)
            setAreasList(areatListTemp)
        }
    }, [areas])

    const handleChange = (event: SelectChangeEvent) => {
        setArea(event.target.value as string)
        doSelectArea(event.target.value as string)
    }

    return (
        <PageHeaderStyles.Wrapper>
            <PageHeaderStyles.Title>{t("areas.common.title")}</PageHeaderStyles.Title>
            <PageHeaderStyles.SubTitle>{t("areas.common.subTitle")}</PageHeaderStyles.SubTitle>

            <FormControl fullWidth>
                <InputLabel>{t("tallysheet.label.area")}</InputLabel>
                <Select
                    value={area || ""}
                    label={t("tallysheet.label.area")}
                    onChange={handleChange}
                >
                    {areasList.map((item) => (
                        <MenuItem key={item.id} value={item.id}>
                            {item.name}
                        </MenuItem>
                    ))}
                </Select>
            </FormControl>

            {/* <ReferenceInput source="area_id" reference="sequent_backend_area_contest">
                        <SelectInput optionText="name" />
                    </ReferenceInput> */}
        </PageHeaderStyles.Wrapper>
    )
}
