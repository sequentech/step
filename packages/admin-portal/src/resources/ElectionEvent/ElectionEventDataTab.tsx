// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {lazy, Suspense} from "react"
import {Alert, Box, CircularProgress} from "@mui/material"
import {useRecordContext} from "react-admin"
import {useTranslation} from "react-i18next"
import type {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {ReadinessCheckButton} from "@/components/election-event/ReadinessCheckButton"
import {EEventDataMode, useElectionEventDataMode} from "./useElectionEventDataMode"

const EditElectionEventData = lazy(() =>
    import("./EditElectionEventData").then((module) => ({default: module.EditElectionEventData}))
)

/** No editor, form, save handler or configuration actions are mounted here. */
export const LockedDownDataTab: React.FC = () => {
    const event = useRecordContext<Sequent_Backend_Election_Event>()
    const mode = useElectionEventDataMode(event)
    const {t} = useTranslation()
    if (mode === EEventDataMode.LOADING) return <CircularProgress aria-label={t("loading")} />
    if (!event || mode !== EEventDataMode.LOCKED_DOWN) {
        return <Alert severity="info">{t("electionEventScreen.dataUnavailable")}</Alert>
    }
    return (
        <Box sx={{display: "flex", justifyContent: "flex-end", flexWrap: "wrap"}}>
            <ReadinessCheckButton key={event.id} event={event} />
        </Box>
    )
}

export const ElectionEventDataTab: React.FC = () => {
    const event = useRecordContext<Sequent_Backend_Election_Event>()
    const mode = useElectionEventDataMode(event)
    const {t} = useTranslation()
    if (mode === EEventDataMode.LOADING) return <CircularProgress aria-label={t("loading")} />
    if (mode === EEventDataMode.UNAVAILABLE) {
        return <Alert severity="info">{t("electionEventScreen.dataUnavailable")}</Alert>
    }
    if (mode === EEventDataMode.LOCKED_DOWN) return <LockedDownDataTab />
    return (
        <Suspense fallback={<CircularProgress aria-label={t("loading")} />}>
            <EditElectionEventData key={event?.id} />
        </Suspense>
    )
}
