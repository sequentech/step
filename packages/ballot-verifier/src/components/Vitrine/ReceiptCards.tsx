// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// What the receipt screen shows of what is being checked: the receipt, and
// the ballot it is checked for (design board VerifierReceipt).

import React from "react"
import Box from "@mui/material/Box"
import {styled} from "@mui/material/styles"
import {useTranslation} from "react-i18next"
import {theme} from "@sequentech/ui-essentials"
import {shortBallotId} from "../../services/vitrine/ballotId"
import {GroupedBallotId} from "./GroupedBallotId"

const MONOSPACE = "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace"

const Card = styled(Box)`
    display: flex;
    gap: 12px;
    padding: 12px 14px;
    border-radius: 4px;
`

const LookedFor = styled(Card)(({theme: muiTheme}) => ({
    alignItems: "flex-start",
    justifyContent: "space-between",
    gap: 16,
    background: theme.palette.green.light,
    [muiTheme.breakpoints.down("sm")]: {
        flexDirection: "column",
        gap: 4,
    },
}))

const Key = styled("div")`
    font-size: 12px;
    line-height: 16px;
    color: ${theme.palette.customGrey.main};
`

const ChangeButton = styled("button")`
    flex: none;
    padding: 4px 0;
    border: 0;
    background: none;
    color: ${theme.palette.brandColor};
    font-family: inherit;
    font-size: 14px;
    font-weight: 500;
    text-decoration: underline;
    text-underline-offset: 3px;
    cursor: pointer;
`

export interface IReceiptFileProps {
    name: string
    /** What the receipt is for, under its name. */
    detail?: string
}

/** The receipt that was opened. */
export const ReceiptFile: React.FC<IReceiptFileProps> = ({name, detail}) => (
    <Card
        data-testid="receipt-file-card"
        sx={{alignItems: "center", border: `1px solid ${theme.palette.customGrey.light}`}}
    >
        <svg
            width="24"
            height="24"
            viewBox="0 0 24 24"
            fill="none"
            stroke={theme.palette.brandColor}
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
            aria-hidden="true"
            style={{flex: "none"}}
        >
            <path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z" />
            <path d="M14 3v5h5" />
            <path d="M9 14.5l2 2 4-4.5" />
        </svg>
        <Box>
            <Box
                data-testid="receipt-file-name"
                sx={{fontSize: 16, lineHeight: "22px", fontWeight: 500, overflowWrap: "anywhere"}}
            >
                {name}
            </Box>
            {detail ? (
                <Box sx={{fontSize: 14, lineHeight: "20px", color: theme.palette.customGrey.main}}>
                    {detail}
                </Box>
            ) : null}
        </Box>
    </Card>
)

export interface IBallotLookedForProps {
    /** The full Ballot ID, in hex. */
    ballotId: string
    /** Lets the user give another Ballot ID; absent when the receipt's link
     * names the ballot. */
    onChange?: () => void
}

/** The ballot the receipt is checked for. */
export const BallotLookedFor: React.FC<IBallotLookedForProps> = ({ballotId, onChange}) => {
    const {t} = useTranslation()
    return (
        <LookedFor data-testid="ballot-looked-for">
            <Box>
                <Key>{t("receiptScreen.ballot.lookedFor")}</Key>
                <Box
                    data-testid="short-ballot-id"
                    sx={{
                        fontFamily: MONOSPACE,
                        fontSize: 22,
                        lineHeight: "30px",
                        fontWeight: 700,
                    }}
                >
                    {shortBallotId(ballotId)}
                </Box>
                <Key sx={{marginTop: "8px"}}>{t("receiptScreen.ballot.fullId")}</Key>
                <GroupedBallotId
                    id={ballotId}
                    data-testid="full-ballot-id"
                    sx={{fontSize: 13, lineHeight: "19px", color: theme.palette.brandColor}}
                />
            </Box>
            {onChange ? (
                <ChangeButton onClick={onChange} data-testid="ballot-change">
                    {t("receiptScreen.ballot.change")}
                </ChangeButton>
            ) : null}
        </LookedFor>
    )
}
