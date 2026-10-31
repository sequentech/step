// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {ballotIdLines} from "../../services/vitrine/ballotId"

export interface IBallotIdLinesProps {
    ballotId: string
    /** Groups of four to a line. */
    perLine: number
}

/** A full Tracking ID in lines, its first two groups, the short ID, in bold.
 * A hyphen ends each line, so that the ID reads and copies as one. */
export const BallotIdLines: React.FC<IBallotIdLinesProps> = ({ballotId, perLine}) => (
    <>
        {ballotIdLines(ballotId, perLine).map((groups, n, lines) => (
            <React.Fragment key={n}>
                {n === 0 ? (
                    <b className="ballot-id-short-groups">{groups.slice(0, 2).join("-")}</b>
                ) : null}
                {n === 0 && groups.length > 2 ? "-" : null}
                {groups.slice(n === 0 ? 2 : 0).join("-")}
                {n < lines.length - 1 ? (
                    <>
                        -<br className="ballot-id-line-break" />
                    </>
                ) : null}
            </React.Fragment>
        ))}
    </>
)
