// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import * as React from "react"
import Button from "@mui/material/Button"
import {useTranslation} from "react-i18next"
import {Box} from "@mui/material"
import {styled} from "@mui/material/styles"

interface VersionProps {
    header?: string
    version: {[key: string]: string}
}

const StyledButton = styled(Button)(({theme}) => ({
    "&.Mui-disabled": {
        borderColor: "transparent",
        opacity: 1,
        color: theme.palette.text.primary, // Use a valid theme property
    },
}))

const Version: React.FC<VersionProps> = ({version, header}) => {
    const {t} = useTranslation()
    const translation = useTranslation()

    return (
        <StyledButton
            variant="actionbar"
            disabled={true}
            sx={{display: {xs: "none", sm: "block"}}}
            className="app-version"
        >
            <Box sx={{width: "100%", display: "flex", flexDirection: "row", alignItems: "center"}}>
                <Box component="span" sx={{display: {xs: "none", md: "block"}}}>
                    {t(header ?? "version.header")}
                </Box>
                <Box component="span">{version["main"]}</Box>
            </Box>
        </StyledButton>
    )
}

export default Version
