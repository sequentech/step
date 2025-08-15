// SPDX-FileCopyrightText: 2023-2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Box, CircularProgress} from "@mui/material"
import {styled} from "@mui/material/styles"

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
    return (
        <StyledBox>
            <CircularProgress />
        </StyledBox>
    )
}

export default Loader
