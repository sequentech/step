// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import Box from "@mui/material/Box"
import Typography from "@mui/material/Typography"
import {styled} from "@mui/material/styles"
import {faCircleQuestion} from "@fortawesome/free-solid-svg-icons"
import React from "react"
// `IBallotStyle` from `./types`, not from `ui-core`: there are two of that name
// and `Question` takes this one — the ui-core shape carries `ballot_eml` and the
// timestamps, which a ballot does not need to draw itself.
import type {BallotSelection, IContest} from "@sequentech/ui-core"

import type {IBallotStyle} from "./types"

import BallotHash from "../components/BallotHash/BallotHash"
import IconButton from "../components/IconButton/IconButton"
import {stringToHtml} from "@sequentech/ui-core"
import {useTranslation} from "react-i18next"

import PageLimit from "../components/PageLimit/PageLimit"
import WarnBox from "../components/WarnBox/WarnBox"
import {theme} from "../services/theme"
import {Question} from "./Question"

const StyledTitle = styled(Typography)`
    margin-top: 25.5px;
    display: flex;
    flex-direction: row;
    gap: 16px;
`

export interface IReviewLayoutProps {
    /**
     * The identifier of the ballot being reviewed, or nothing to leave the block
     * out — which is what an audit configuration of `NOT_SHOW` asks for, and also
     * what a preview needs, because no ballot has been cast to have one.
     */
    ballotId?: string
    /** Named as `BallotHash` names it, since that is where they end up. */
    onBallotIdHelp?: () => void

    /**
     * The breadcrumb, supplied rather than built.
     *
     * The portal's own `Stepper` reads the store to decide whether the election
     * list counts as a step; a preview has no store and a different set of steps.
     * Neither belongs in here.
     */
    steps?: React.ReactNode

    onTitleHelp?: () => void

    /** A failure to show above the contests — casting refused, usually. */
    error?: React.ReactNode

    /** Already-resolved copy: the caller decides which of its variants applies. */
    /**
     * Whether this event offers *Audit ballot*.
     *
     * Not wording: it chooses between `reviewScreen.description` and
     * `.descriptionNoAudit`, because one of them mentions a button the other event does
     * not have. The portal reads `EVotingPortalAuditButtonCfg` and a preview reads the
     * plan, and that is a policy neither this layout nor a translator should decide.
     */
    withAudit?: boolean

    ballotStyle: IBallotStyle
    contests: IContest[]
    errorSelectionState: BallotSelection
    isDeclineToVote?: boolean
    /**
     * The voter left the whole ballot empty under a policy that allows it.
     *
     * Beside {@link isDeclineToVote} because it is the same kind of fact — one
     * about the ballot rather than about a contest — and `Question` reads both to
     * label a contest that carries no choice.
     */
    isBlankBallot?: boolean

    /** What sits under the contests: cast, go back, audit. */
    actions?: React.ReactNode

    /**
     * Anything the host owns that has to render inside the page — its dialogs,
     * in practice. They are behaviour, and behaviour stays with whoever has the
     * state that drives it.
     */
    children?: React.ReactNode
}

/**
 * What the review screen looks like, with nothing about what it does.
 *
 * **Extracted so the Election Architect's ballot preview can show the same
 * screen rather than a drawing of it.** The preview already renders the voting
 * screen from `Question`; review was the one it could not reach, because the
 * arrangement around the contests — the ballot identifier, the breadcrumb, the
 * title and its help, the description that changes with the audit setting —
 * lived only inside an 868-line route bound to Redux and Apollo.
 *
 * Copying it into the wizard would have worked once and then drifted, and the
 * drift would be invisible: a preview is only useful for being accurate, and
 * nothing fails when it stops being. So the arrangement moved here and both
 * sides render it.
 *
 * **Props, not state.** Everything conditional is decided by the caller: the
 * portal knows about `EVotingPortalAuditButtonCfg` and the preview knows there
 * is no cast ballot, and neither fact belongs to a layout. What it keeps is the
 * order things appear in and the spacing between them, which is the part that
 * has to match.
 */
export const ReviewLayout: React.FC<IReviewLayoutProps> = ({
    ballotId,
    onBallotIdHelp,
    steps,
    onTitleHelp,
    error,
    withAudit = false,
    ballotStyle,
    contests,
    errorSelectionState,
    isDeclineToVote,
    isBlankBallot,
    actions,
    children,
}) => {
    const {t} = useTranslation()

    /*
     * `reviewScreen.*`, translated here.
     *
     * The heading and the description arrived as props until `EA-F3-015`, which meant
     * whoever called this decided what the screen said — and the Election Architect's
     * preview called it with wording of its own, so a client who overrode
     * `reviewScreen.title` saw their words in the portal and "Review your ballot" in the
     * picture that is supposed to be the portal. Every string on this screen is
     * `voting-portal/src/translations/<lng>.ts`, on the paths clients override.
     */
    const description = withAudit
        ? t("reviewScreen.description")
        : t("reviewScreen.descriptionNoAudit")

    return (
        <PageLimit maxWidth="lg" className="review-screen screen">
            {ballotId === undefined ? null : (
                <BallotHash
                    hash={ballotId}
                    copyLabels={{
                        copy: t("reviewScreen.copyBallotId"),
                        copied: t("reviewScreen.ballotIdCopied"),
                        error: t("reviewScreen.ballotIdCopyError"),
                    }}
                    helpButtonLabel={t("reviewScreen.ballotIdHelpDialog.title")}
                    onHelpClick={onBallotIdHelp}
                />
            )}
            {children}
            {steps === undefined ? null : <Box marginTop="48px">{steps}</Box>}
            <StyledTitle variant="h4" fontSize="24px" fontWeight="bold" sx={{margin: 0}}>
                <Box>{t("reviewScreen.title")}</Box>
                {onTitleHelp === undefined ? null : (
                    <IconButton
                        icon={faCircleQuestion}
                        sx={{
                            fontSize: "unset",
                            lineHeight: "unset",
                            paddingBottom: "2px",
                        }}
                        fontSize="16px"
                        onClick={onTitleHelp}
                    />
                )}
            </StyledTitle>
            {error ? <WarnBox variant="error">{error}</WarnBox> : null}
            <Typography variant="body2" sx={{color: theme.palette.customGrey.main}}>
                {stringToHtml(description)}
            </Typography>
            {contests.map((question, index) => (
                <Box key={question.id} className={`contest-${index}`}>
                    <Question
                        ballotStyle={ballotStyle}
                        question={question}
                        isReview={true}
                        setDecodedContests={() => undefined}
                        errorSelectionState={errorSelectionState}
                        isDeclineToVote={isDeclineToVote}
                        isBlankBallot={isBlankBallot}
                    />
                </Box>
            ))}
            {actions}
        </PageLimit>
    )
}
