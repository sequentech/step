// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useRef} from "react"
import Paper from "@mui/material/Paper"
import {faCloudArrowUp} from "@fortawesome/free-solid-svg-icons"
import {useTranslation} from "react-i18next"
import {Box, Typography} from "@mui/material"
import {useInput} from "react-admin"
import {CustomDropFile, Icon, theme} from "@sequentech/ui-essentials"
import InboxIcon from "@mui/icons-material/Inbox"

interface NoItemProps {
    item?: string
}

export const NoItem: React.FC<NoItemProps> = ({item}) => {
    const {t} = useTranslation()

    return (
        <Paper
            variant="responsive"
            sx={{width: "100%", gap: "7px", padding: "16px", backgroundColor: "inherit"}}
        >
            <InboxIcon sx={{fontSize: "12rem", color: theme.palette.customGrey.dark}} />
            
            <Box>
                <Typography
                    sx={{
                        margin: 0,
                        fontSize: "2rem",
                        color: theme.palette.customGrey.dark,
                    }}
                >
                    {t("common.label.noResult")}
                </Typography>
            </Box>
        </Paper>
    )
}
