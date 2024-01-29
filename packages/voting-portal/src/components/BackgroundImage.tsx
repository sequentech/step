// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Box} from "@mui/material"
import {styled} from "@mui/material/styles"

export const BackgroundImage = styled(Box)<{imgurl: string}>`
    background-image: linear-gradient(1deg, #ffffff, transparent), url(${({imgurl}) => imgurl}) !important;
    background-size: cover;
    position: absolute;
    top: 0;
    bottom: 0;
    left: 0;
    right: 0;
    z-index: -1;
`
