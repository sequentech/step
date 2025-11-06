// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {styled} from "@mui/material/styles"
import {Box} from "@mui/material"

const InfoDataBox = styled(Box)`
    word-break: break-word;
    hyphens: auto;
    padding: 15px;
    background-color: #ecfdf5;
    color: #000;
    border-radius: 4px;
    display: block;
    overflow-y: scroll;
    max-height: 200px;
    border: 1px solid #047857;
    margin: 4px 0;
`

export default InfoDataBox
