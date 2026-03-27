// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    Sequent_Backend_Election,
    Sequent_Backend_Tally_Sheet,
    Sequent_Backend_Tally_Sheet_Insert_Input,
} from "@/gql/graphql"
import React from "react"
import {TallySheetsDataFields} from "./TallySheetsDataFields"
import {useTallySheetsDataState} from "./hooks/useTallySheetDataState"
import {Box, TextField} from "@mui/material"
import {PageHeaderStyles} from "@/components/styles/PageHeaderStyles"
import {useGetOne} from "react-admin"
import {useTranslation} from "react-i18next"

interface TallySheetReviewProps {
    tallySheet: Sequent_Backend_Tally_Sheet | Sequent_Backend_Tally_Sheet_Insert_Input
    election: Sequent_Backend_Election
}

export const TallySheetReview: React.FC<TallySheetReviewProps> = (props) => {
    const {tallySheet, election} = props
    const {t} = useTranslation()
    const {results, invalids, candidatesResults} = useTallySheetsDataState({
        election,
        tallySheet,
        editable: false,
    })

    const {data: area} = useGetOne(
        "sequent_backend_area",
        {
            id: tallySheet.area_id,
            meta: {tenant_id: election.tenant_id, election_event_id: election.election_event_id},
        },
        {enabled: !!tallySheet.area_id}
    )

    const {data: contest} = useGetOne(
        "sequent_backend_contest",
        {
            id: tallySheet.contest_id,
            meta: {tenant_id: election.tenant_id, election_event_id: election.election_event_id},
        },
        {enabled: !!tallySheet.contest_id}
    )

    return (
        <>
            <Box>
                <Box>
                    <PageHeaderStyles.Title>{t("tallysheet.common.title")}</PageHeaderStyles.Title>
                    <PageHeaderStyles.SubTitle>
                        {t("tallysheet.common.subtitle")}
                    </PageHeaderStyles.SubTitle>
                    <TextField
                        label="Area"
                        value={area?.name ?? ""}
                        size="small"
                        fullWidth
                        disabled
                    />
                    <TextField
                        label="Contest"
                        value={contest?.name ?? ""}
                        size="small"
                        fullWidth
                        disabled
                    />
                    <TextField
                        label={t("tallysheet.label.channel")}
                        value={tallySheet.channel}
                        size="small"
                        fullWidth
                        disabled
                    />
                </Box>
                <TallySheetsDataFields
                    results={results}
                    invalids={invalids}
                    candidatesResults={candidatesResults}
                    isEditable={false}
                />
            </Box>
        </>
    )
}
