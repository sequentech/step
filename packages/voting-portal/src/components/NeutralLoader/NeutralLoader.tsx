// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Box, CircularProgress} from "@mui/material"
import {styled} from "@mui/material/styles"
import {useTranslation} from "react-i18next"

// Shown before the election event's presentation is known, so it must not
// carry the default theme's colours.
export const NEUTRAL_LOADER_COLOR = "#767676"

const StyledBox = styled(Box)`
    display: flex;
    flex: 1;
    min-height: 100vh;
    align-items: center;
    justify-content: center;
    background-color: #fff;
    color: ${NEUTRAL_LOADER_COLOR};
`

const NeutralLoader: React.FC = () => {
    const {t} = useTranslation()

    return (
        <StyledBox className="neutral-loader" role="status">
            <CircularProgress
                className="neutral-loader-progress"
                color="inherit"
                aria-label={t("a11y.loading")}
            />
        </StyledBox>
    )
}

export default NeutralLoader
