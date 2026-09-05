// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import * as React from "react"
import {useTranslation} from "react-i18next"
import {Box, Typography} from "@mui/material"
import {styled} from "@mui/material/styles"

interface VersionProps {
    header?: string
    version: {[key: string]: string}
}

const StyledVersion = styled(Typography)<{component?: React.ElementType}>(({theme}) => ({
    boxSizing: "border-box",
    minWidth: "64px",
    minHeight: "44px",
    padding: "6px 12px",
    color: theme.palette.brandColor,
    backgroundColor: "rgba(255, 255, 255, 0.4)",
}))

const Version: React.FC<VersionProps> = ({version, header}) => {
    const {t} = useTranslation()

    return (
        <StyledVersion
            component="div"
            variant="button"
            sx={{display: {xs: "none", sm: "block"}}}
            className="app-version"
        >
            <Box sx={{width: "100%", display: "flex", flexDirection: "row", alignItems: "center"}}>
                <Box component="span" sx={{display: {xs: "none", md: "block"}}}>
                    {t(header ?? "version.header")}
                </Box>
                <Box component="span">{version["main"]}</Box>
            </Box>
        </StyledVersion>
    )
}

export default Version
