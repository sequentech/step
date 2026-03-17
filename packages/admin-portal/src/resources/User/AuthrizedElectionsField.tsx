// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {useRecordContext, useGetList} from "react-admin"
import {Chip, Stack} from "@mui/material"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import {useTranslation} from "react-i18next"
import {AUTHORIZED_ELECTION_IDS} from "./ListUsers"

export const AuthorizedElectionsField = ({electionEventId}: {electionEventId?: string}) => {
    const {t} = useTranslation()
    const record = useRecordContext()

    const aliasRenderer = useAliasRenderer()
    const ids: string[] = record?.attributes?.[AUTHORIZED_ELECTION_IDS] ?? []

    const enabled = !!electionEventId && ids.length > 0

    const {data} = useGetList(
        "sequent_backend_election",
        {
            filter: {
                election_event_id: electionEventId,
                alias: {
                    format: "hasura-raw-query",
                    value: {_in: ids},
                },
            },
            pagination: {page: 1, perPage: 500},
            sort: {field: "id", order: "DESC"},
        },
        {enabled}
    )

    if (!enabled) return null

    return (
        <Stack direction="row" spacing={1} useFlexGap flexWrap="wrap">
            {data?.map((e: any) => (
                <Chip key={e.id} label={aliasRenderer(e) ?? ""} />
            ))}
        </Stack>
    )
}
