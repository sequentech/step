// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect} from "react"
import {useRecordContext} from "react-admin"
import {Box, Button} from "@mui/material"
import {useTranslation} from "react-i18next"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {ElectionHeaderStyles} from "@/components/styles/ElectionHeaderStyles"
import {useIvrEmulator} from "@/providers/IvrEmulatorContextProvider"

export const IvrEmulator: React.FC = () => {
    const {t} = useTranslation()
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const {status: emulatorStatus, api} = useIvrEmulator()

    useEffect(() => {
        if (api) {
            api.welcome()
        }
    }, [api])
    if (!record?.id) {
        return null
    }

    return (
        <Box sx={{display: "flex", flexDirection: "column", gap: 2}}>
            <ElectionHeaderStyles.SubTitle>
                {t("electionEventScreen.ivr.config.infoMsg")}
            </ElectionHeaderStyles.SubTitle>

            <div>`{emulatorStatus}`</div>

            <Box sx={{mt: 3, display: "flex", gap: 2}}>
                <Button variant="contained">{t("common.label.save")}</Button>
                <Button variant="contained">{t("common.label.cancel")}</Button>
            </Box>
        </Box>
    )
}
