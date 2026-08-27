// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Box, CircularProgress} from "@mui/material"
import {styled} from "@mui/material/styles"
import {useTranslation} from "react-i18next"
import VisuallyHidden from "../VisuallyHidden/VisuallyHidden"

const StyledBox = styled(Box)`
    display: flex;
    position: absolute;
    top: 0;
    bottom: 0;
    right: 0;
    left: 0;
    margin: auto;
    align-items: center;
    justify-content: center;
`

const Loader = () => {
    const {t} = useTranslation()

    return (
        <StyledBox className="loader" role="status">
            <CircularProgress aria-hidden="true" />
            <VisuallyHidden>{t("a11y.loading")}</VisuallyHidden>
        </StyledBox>
    )
}

export default Loader
