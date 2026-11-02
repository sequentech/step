// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// What the ballot tracker shows under the record of an audited ballot (design
// boards TrackerAudit and PhoneTrackerAudit): the selections the ballot decodes
// into once its audit has passed, for the voter to compare with what they
// chose (AuditCompare), or else why there are none. The selections are drawn
// from memory and kept nowhere, but for the evidence file a voter chooses to
// save.

import React from "react"
import {useTranslation} from "react-i18next"
import {Box, Typography} from "@mui/material"
import Image from "mui-image"
import {BlankAnswer, Candidate} from "@sequentech/ui-essentials"
import {stringToHtml, translate} from "@sequentech/ui-core"
import type {IBallotStyle, ICandidate, IContest, IDecodedVoteContest} from "@sequentech/ui-core"
import {provideBallotService} from "../../services/BallotService"
import {checkIsInvalidVote, getImageUrl} from "../../services/ElectionConfigService"
import {
    auditEvidence,
    evidenceFileName,
    EVIDENCE_MEDIA_TYPE,
    IAuthorityContact,
} from "../../services/vitrine/auditEvidence"
import {EAuditOutcome, TAudit, TAuditedRecord} from "../../services/vitrine/ballotAudit"
import {saveTextFile} from "../../services/vitrine/file"
import {AuditCompare} from "./AuditCompare"
import {PALETTE, PHONE} from "./trackerStyle"

const RANK_LAST = Number.MAX_SAFE_INTEGER

const CARD_SX = {
    marginTop: "24px",
    padding: "24px",
    border: `1px solid ${PALETTE.border}`,
    borderRadius: "4px",
    [PHONE]: {padding: "20px 16px"},
} as const

const HEADING_SX = {margin: 0, fontSize: 20, lineHeight: "28px", fontWeight: 700} as const
const NOTE_SX = {fontSize: 14, lineHeight: "20px", color: PALETTE.muted} as const

interface IChoice {
    candidate: ICandidate
    /** Where a ranked contest puts the candidate, from 1. */
    position?: number
    /** What the voter wrote in. */
    writeIn?: string
}

/** The candidates a decoded contest chose, in the contest's order, or in the
 * order of the voter's ranking. */
const choicesOf = (contest: IContest, decoded: IDecodedVoteContest, ranked: boolean): IChoice[] => {
    const chosen = contest.candidates.flatMap((candidate): IChoice[] => {
        if (checkIsInvalidVote(candidate)) return decoded.is_explicit_invalid ? [{candidate}] : []
        const choice = decoded.choices.find(({id}) => id === candidate.id)
        if (!choice || choice.selected < 0) return []
        return [
            {
                candidate,
                position: choice.selected + 1,
                ...(choice.write_in_text ? {writeIn: choice.write_in_text} : {}),
            },
        ]
    })
    return ranked
        ? [...chosen].sort((a, b) => (a.position ?? RANK_LAST) - (b.position ?? RANK_LAST))
        : chosen
}

interface IDecodedContestProps {
    contest: IContest
    decoded: IDecodedVoteContest
    language: string
    publicBucketUrl: string
}

const DecodedContest: React.FC<IDecodedContestProps> = ({
    contest,
    decoded,
    language,
    publicBucketUrl,
}) => {
    const {t} = useTranslation()
    const {isPreferential} = provideBallotService()
    const ranked = isPreferential(contest.counting_algorithm)
    const choices = choicesOf(contest, decoded, ranked)
    const headingId = `tracker-selection-${contest.id}-title`

    return (
        <Box component="section" className="tracker-selection" aria-labelledby={headingId}>
            <Typography
                component="h3"
                id={headingId}
                className="tracker-selection-title"
                sx={{margin: "0 0 8px", fontSize: 14, lineHeight: "20px", fontWeight: 700}}
            >
                {translate(contest, "name", language)}
            </Typography>
            <Box
                component="ul"
                className="tracker-selection-choices"
                sx={{
                    listStyle: "none",
                    margin: 0,
                    padding: 0,
                    display: "flex",
                    flexDirection: "column",
                    gap: "12px",
                }}
            >
                {decoded.is_decline_to_vote ? (
                    <BlankAnswer title={t("reviewScreen.declineToVote")} />
                ) : choices.length === 0 ? (
                    <BlankAnswer />
                ) : (
                    choices.map(({candidate, position, writeIn}) => {
                        const imageUrl = getImageUrl(candidate)
                        return (
                            <Candidate
                                key={candidate.id}
                                title={translate(candidate, "name", language)}
                                description={
                                    writeIn ??
                                    stringToHtml(
                                        translate(candidate, "description", language) || ""
                                    )
                                }
                                isSelectable={false}
                                isInvalidVote={checkIsInvalidVote(candidate)}
                                isPreferentialVote={ranked}
                                selectedPosition={position}
                                totalCandidates={contest.candidates.length}
                                maxVotes={contest.max_votes}
                                checked={true}
                            >
                                {imageUrl ? (
                                    <Image
                                        className="candidate-image"
                                        src={`${publicBucketUrl}${imageUrl}`}
                                        duration={100}
                                    />
                                ) : null}
                            </Candidate>
                        )
                    })
                )}
            </Box>
        </Box>
    )
}

export interface ITrackerAuditProps {
    audit: TAudit
    /** The ballot style that the audited ballot carries. */
    ballotStyle: IBallotStyle
    language: string
    publicBucketUrl: string
    /** The audited ballot's full Tracking ID. */
    ballotId: string
    /** The audited ballot, as the public record holds it. */
    record: TAuditedRecord
    /** Where the voter goes to vote, once the ballot matches what they chose. */
    voteHref: string
    /** Who a voter whose ballot does not match reports it to, when known. */
    contact?: IAuthorityContact
}

export const TrackerAudit: React.FC<ITrackerAuditProps> = ({
    audit,
    ballotStyle,
    language,
    publicBucketUrl,
    ballotId,
    record,
    voteHref,
    contact,
}) => {
    const {t} = useTranslation()

    if (audit.outcome === EAuditOutcome.MISMATCH) {
        return (
            <Box
                component="section"
                role="alert"
                className="tracker-audit-failed"
                sx={{
                    ...CARD_SX,
                    borderColor: "#FECACA",
                    backgroundColor: "#FEF2F2",
                    color: "#7F1D1D",
                }}
            >
                <Typography
                    component="h2"
                    className="tracker-audit-failed-title"
                    sx={{...HEADING_SX, color: PALETTE.redInk}}
                >
                    {t("ballotLocator.audit.failed.title")}
                </Typography>
                <Typography
                    className="tracker-audit-failed-text"
                    sx={{...NOTE_SX, margin: "8px 0 0", color: "inherit"}}
                >
                    {t("ballotLocator.audit.failed.text")}
                </Typography>
            </Box>
        )
    }

    if (audit.outcome === EAuditOutcome.NOT_VERIFIABLE) {
        return (
            <Box
                component="section"
                role="status"
                className="tracker-audit-unchecked"
                sx={{...CARD_SX, backgroundColor: PALETTE.tint}}
            >
                <Typography
                    component="h2"
                    className="tracker-audit-unchecked-title"
                    sx={HEADING_SX}
                >
                    {t("ballotLocator.audit.unchecked.title")}
                </Typography>
                <Typography
                    className="tracker-audit-unchecked-text"
                    sx={{...NOTE_SX, margin: "8px 0 0"}}
                >
                    {t("ballotLocator.audit.unchecked.text")}
                </Typography>
            </Box>
        )
    }

    const contests = ballotStyle.contests.flatMap((contest) => {
        const decoded = audit.selections.find(({contest_id}) => contest_id === contest.id)
        return decoded ? [{contest, decoded}] : []
    })

    const saveEvidence = () =>
        saveTextFile(
            evidenceFileName(ballotId),
            auditEvidence({ballotId, record, selections: audit.selections}),
            EVIDENCE_MEDIA_TYPE
        )

    return (
        <AuditCompare
            ballotId={ballotId}
            voteHref={voteHref}
            contact={contact}
            onSaveEvidence={saveEvidence}
        >
            <Box
                className="tracker-selections-list"
                sx={{
                    display: "grid",
                    gridTemplateColumns: "repeat(auto-fit, minmax(260px, 1fr))",
                    gap: "16px 20px",
                }}
            >
                {contests.map(({contest, decoded}) => (
                    <DecodedContest
                        key={contest.id}
                        contest={contest}
                        decoded={decoded}
                        language={language}
                        publicBucketUrl={publicBucketUrl}
                    />
                ))}
            </Box>
        </AuditCompare>
    )
}
