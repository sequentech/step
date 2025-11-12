// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {styled} from "@mui/material/styles"
import {theme} from "@sequentech/ui-essentials"
import React from "react"

export interface JsonViewProps {
    origin: object
}

export const JsonView: React.FC<JsonViewProps> = (props) => {
    const {origin} = props

    const AreaView = styled("div")`
        display: flex;
        width: 100%;
        background-color: ${theme.palette.lightBackground};
        padding: 1rem 2rem;
        font-size: 0.8rem;
        max-height: 300px;
        overflow-y: scroll;
    `

    const AreaText = styled("pre")`
        white-space: pre-wrap;
        white-space: -moz-pre-wrap;
        white-space: -pre-wrap;
        white-space: -o-pre-wrap;
        word-wrap: break-word;
    `

    return (
        <AreaView>
            <AreaText>{JSON.stringify(origin, null, 8)}</AreaText>
        </AreaView>
    )
}
