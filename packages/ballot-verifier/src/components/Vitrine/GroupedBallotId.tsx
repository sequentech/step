// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// A Tracking ID in groups of four, the first two in bold: the short ID. Given
// another ID to compare with, it highlights the groups that differ. It wraps
// by its width in characters: two lines on wide screens, four on phones.

import React from "react"
import type {SxProps, Theme} from "@mui/material/styles"
import {styled} from "@mui/material/styles"
import {ballotIdGroups, SHORT_BALLOT_ID_GROUPS} from "../../services/vitrine/ballotId"

const ID_WIDTH_WIDE = "40ch"
const ID_WIDTH_NARROW = "20ch"
const DIFFERENT = "#FCA5A5"

const Id = styled("span")(({theme}) => ({
    display: "block",
    width: ID_WIDTH_WIDE,
    maxWidth: "100%",
    fontFamily: "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace",
    fontSize: 15,
    lineHeight: "22px",
    [theme.breakpoints.down("sm")]: {
        width: ID_WIDTH_NARROW,
    },
}))

const Different = styled("mark")`
    border-radius: 2px;
    background: ${DIFFERENT};
    color: inherit;
    box-shadow: 0 0 0 1px ${DIFFERENT};
`

export interface IGroupedBallotIdProps {
    "id": string
    /** The ID to compare with: the groups that differ from it are highlighted. */
    "compareTo"?: string
    "sx"?: SxProps<Theme>
    "data-testid"?: string
}

export const GroupedBallotId: React.FC<IGroupedBallotIdProps> = ({
    id,
    compareTo,
    sx,
    "data-testid": testId,
}) => {
    const groups = ballotIdGroups(id)
    const other = compareTo === undefined ? undefined : ballotIdGroups(compareTo)
    return (
        <Id sx={sx} data-testid={testId}>
            {groups.map((group, k) => {
                const shown =
                    other !== undefined && other[k] !== group ? (
                        <Different>{group}</Different>
                    ) : (
                        group
                    )
                return (
                    <React.Fragment key={k}>
                        {k < SHORT_BALLOT_ID_GROUPS ? <b>{shown}</b> : shown}
                        {k < groups.length - 1 ? " " : null}
                    </React.Fragment>
                )
            })}
        </Id>
    )
}
