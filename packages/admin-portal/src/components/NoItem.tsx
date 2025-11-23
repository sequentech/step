// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import Paper from "@mui/material/Paper"
import {useTranslation} from "react-i18next"
import {Box, Typography} from "@mui/material"
import InboxIcon from "@mui/icons-material/Inbox"
import {theme} from "@sequentech/ui-essentials"

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
            <InboxIcon sx={{fontSize: "8rem", color: theme.palette.customGrey.dark}} />

            <Box>
                <Typography
                    sx={{
                        margin: 0,
                        fontSize: "2rem",
                        color: theme.palette.customGrey.dark,
                    }}
                >
                    {item ? item : t("common.label.noResult")}
                </Typography>
            </Box>
        </Paper>
    )
}
