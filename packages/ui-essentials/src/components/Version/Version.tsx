// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import * as React from "react"
import Button from "@mui/material/Button"
import {useTranslation} from "react-i18next"
import {Box} from "@mui/material"

interface VersionProps {
    version: {[key: string]: string}
}

const Version: React.FC<VersionProps> = ({version}) => {
    const {t, i18n} = useTranslation()

    return (
        <Button variant="actionbar">
            <Box component="span" sx={{display: {xs: "none", lg: "block"}}}>
                {t("version.header")}
            </Box>
            <Box component="span">{version["main"]}</Box>
        </Button>
    )
}

export default Version
