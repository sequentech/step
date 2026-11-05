// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// "Check your audited ballot" (design boards AuditScreen and PhoneAuditScreen),
// for an event that keeps a transparency log: the review screen's Tracking ID
// box, now Audited, and three steps. The last lists the hubs of the election's
// policy that run a ballot verifier, as equal tiles in a random order. Each
// opens the hub's own site with the Tracking ID in the link's fragment; the
// ballot and its secret come from the public record, not from this page.

import React, {useId, useState} from "react"
import {Box, Button} from "@mui/material"
import {styled} from "@mui/material/styles"
import {useTranslation} from "react-i18next"
import {Link as RouterLink} from "react-router-dom"
import {faAngleRight, faCircleQuestion} from "@fortawesome/free-solid-svg-icons"
import type {Policy} from "@vitrine/verify"
import {Icon, IconButton, theme} from "@sequentech/ui-essentials"
import {EAuditPublishState} from "../../hooks/useAuditBallot"
import {groupBallotId} from "../../services/vitrine/ballotId"
import type {IReceiptView} from "../../services/vitrine/castReceipt"
import {auditedBallotLink, auditHubs, shuffled} from "../../services/vitrine/hubUpdates"
import {clockMinutes} from "../../services/vitrine/statusRows"
import {useTrackerLink} from "./BallotStatus"
import {ExternalIcon} from "./GetUpdates"
import {EReviewBallotIdState, ReviewBallotId} from "./ReviewBallotId"

const COLORS = {
    navy: "#0F054C",
    text: "#191D23",
    body: "#475569",
    quiet: "#64748B",
    tip: "#334155",
    line: "#E7EAEE",
    rule: "#EEF1F5",
    now: "#F7F9FE",
    tile: "#CBD3E3",
    mark: "#E9EAF6",
    failed: "#B42318",
    failedNow: "#FEF3F2",
} as const

const PHONE = theme.breakpoints.down("sm")

const KEYS = "auditScreen.check"

/** Where the title row stacks, as on the review screen. */
const TITLE_ROW_STACKED_MAX_PX = 900

/** The steps, the title and the Tracking ID box, laid out as the review
 * screen's `ReviewHeader` in `ui-essentials` lays them out. */
const AuditHeader = styled(Box)`
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    grid-template-areas:
        "steps steps steps"
        "title id .";
    column-gap: 16px;
    align-items: center;

    .audit-header-steps {
        grid-area: steps;
        margin-top: 48px;
    }

    .audit-header-title {
        grid-area: title;
    }

    .audit-header-id {
        grid-area: id;
        padding: 32px 0 16px;
    }

    @media (min-width: ${TITLE_ROW_STACKED_MAX_PX + 1}px) {
        .audit-header-title .screen-title {
            white-space: nowrap;
        }
    }

    @media (max-width: ${TITLE_ROW_STACKED_MAX_PX}px) {
        grid-template-columns: 1fr;
        grid-template-areas:
            "steps"
            "title"
            "id";

        .audit-header-id {
            justify-self: center;
            padding: 12px 0 16px;
        }
    }

    ${({theme}) => theme.breakpoints.down("sm")} {
        grid-template-areas:
            "id"
            "steps"
            "title";

        .audit-header-steps {
            margin-top: 16px;
        }

        .audit-header-title .screen-title {
            padding: 16px 0 8px;
        }

        .audit-header-id {
            justify-self: stretch;
            padding: 0;
        }
    }
`

/** Up to two letters for a hub's tile, from the words of its name. */
export const hubMonogram = (name: string): string =>
    name
        .split(/[\s_-]+/)
        .filter(Boolean)
        .slice(0, 2)
        .map((word) => Array.from(word)[0])
        .join("")

const Done: React.FC = () => (
    <svg
        className="audit-step-done-icon"
        width="14"
        height="14"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="3.4"
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
    >
        <path className="audit-step-done-path" d="M4.5 12.5l4.75 4.75L19.5 7" />
    </svg>
)

const PhoneIcon: React.FC = () => (
    <svg
        className="audit-tip-icon"
        width="16"
        height="16"
        viewBox="0 0 24 24"
        fill="none"
        stroke={COLORS.navy}
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
        style={{flex: "none", marginTop: 1}}
    >
        <rect className="audit-tip-icon-body" x="7" y="2.5" width="10" height="19" rx="2" />
        <path className="audit-tip-icon-button" d="M11 18.5h2" />
    </svg>
)

const Chevron: React.FC<{open: boolean}> = ({open}) => (
    <svg
        className="audit-why-chevron"
        width="14"
        height="14"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2.4"
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
        style={{transform: `rotate(${open ? 180 : 0}deg)`}}
    >
        <path className="audit-why-chevron-path" d="M6 9l6 6 6-6" />
    </svg>
)

const stepSx = {
    display: "grid",
    gridTemplateColumns: "28px minmax(0, 1fr) auto",
    columnGap: "16px",
    alignItems: "start",
    p: "18px 22px",
    borderTop: `1px solid ${COLORS.rule}`,
    [PHONE]: {gridTemplateColumns: "28px minmax(0, 1fr)", columnGap: "12px", p: "16px 14px"},
} as const

const numberSx = {
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    width: 28,
    height: 28,
    boxSizing: "border-box",
    borderRadius: "50%",
    fontSize: "14px",
    fontWeight: 700,
} as const

const whenSx = {
    fontSize: "13px",
    lineHeight: "28px",
    color: COLORS.quiet,
    whiteSpace: "nowrap",
    [PHONE]: {gridColumn: 2, lineHeight: "18px"},
} as const

/** What spans the step's text and time, under them; the whole row on a phone. */
const underSx = {gridColumn: "2 / -1", [PHONE]: {gridColumn: "1 / -1"}} as const

enum EStepState {
    DONE = "done",
    CURRENT = "current",
    FAILED = "failed",
    /** Not reached yet. */
    NEXT = "next",
}

const numberStateSx = {
    [EStepState.DONE]: {bgcolor: COLORS.navy, color: "#fff"},
    [EStepState.CURRENT]: {
        bgcolor: "#fff",
        border: `1.5px solid ${COLORS.navy}`,
        color: COLORS.navy,
    },
    [EStepState.FAILED]: {
        bgcolor: "#fff",
        border: `1.5px solid ${COLORS.failed}`,
        color: COLORS.failed,
    },
    [EStepState.NEXT]: {bgcolor: "#fff", border: `1.5px solid ${COLORS.tile}`, color: COLORS.quiet},
} as const

const rowStateSx = {
    [EStepState.DONE]: {},
    [EStepState.CURRENT]: {bgcolor: COLORS.now},
    [EStepState.FAILED]: {bgcolor: COLORS.failedNow},
    [EStepState.NEXT]: {},
} as const

interface IStepProps {
    className: string
    title: string
    text: string
    when?: string
    state: EStepState
    number: number
    children?: React.ReactNode
}

const Step: React.FC<IStepProps> = ({className, title, text, when, state, number, children}) => (
    <Box
        component="li"
        className={`audit-step ${className} audit-step-${state}`}
        aria-current={
            state === EStepState.CURRENT || state === EStepState.FAILED ? "step" : undefined
        }
        sx={[stepSx, rowStateSx[state], {"&:first-of-type": {borderTop: 0}}]}
    >
        <Box
            component="span"
            className="audit-step-number"
            sx={{...numberSx, ...numberStateSx[state]}}
        >
            {state === EStepState.DONE ? <Done /> : state === EStepState.FAILED ? "!" : number}
        </Box>
        <Box className="audit-step-text">
            <Box
                className="audit-step-title"
                sx={{fontSize: "16px", lineHeight: "28px", fontWeight: 700}}
            >
                {title}
            </Box>
            <Box
                className="audit-step-detail"
                sx={{mt: "2px", fontSize: "14px", lineHeight: "20px", color: COLORS.quiet}}
            >
                {text}
            </Box>
        </Box>
        <Box component="span" className="audit-step-when" sx={whenSx}>
            {when ?? ""}
        </Box>
        {children}
    </Box>
)

interface IAuditTile {
    name: string
    who: string
    label: string
    href: string
}

const HubTile: React.FC<{tile: IAuditTile}> = ({tile}) => (
    <Box component="li" className="audit-hub-item" sx={{display: "flex"}}>
        <Box
            component="a"
            className="audit-hub"
            href={tile.href}
            target="_blank"
            rel="noopener noreferrer"
            aria-label={tile.label}
            sx={{
                "flex": 1,
                "minWidth": 0,
                "display": "flex",
                "alignItems": "center",
                "gap": "12px",
                "minHeight": 60,
                "p": "10px 14px",
                "boxSizing": "border-box",
                "border": `1px solid ${COLORS.tile}`,
                "borderRadius": "8px",
                "bgcolor": "#fff",
                "color": COLORS.navy,
                "textDecoration": "none",
                "transition": "border-color .15s, box-shadow .15s",
                "&:hover": {borderColor: COLORS.navy, boxShadow: "0 1px 0 rgba(15, 5, 76, 0.08)"},
                "&:focus-visible": {outline: `2px solid ${COLORS.navy}`, outlineOffset: "2px"},
            }}
        >
            <Box
                component="span"
                className="audit-hub-mark"
                aria-hidden="true"
                sx={{
                    flex: "none",
                    width: 36,
                    height: 36,
                    borderRadius: "8px",
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                    bgcolor: COLORS.mark,
                    color: COLORS.navy,
                    fontSize: "13px",
                    fontWeight: 700,
                }}
            >
                {hubMonogram(tile.name)}
            </Box>
            <Box component="span" className="audit-hub-text" sx={{flex: 1, minWidth: 0}}>
                <Box
                    component="span"
                    className="audit-hub-name"
                    sx={{
                        display: "block",
                        fontSize: "15px",
                        lineHeight: "20px",
                        fontWeight: 600,
                        overflowWrap: "anywhere",
                    }}
                >
                    {tile.name}
                </Box>
                <Box
                    component="span"
                    className="audit-hub-who"
                    sx={{
                        display: "block",
                        fontSize: "13px",
                        lineHeight: "18px",
                        color: COLORS.quiet,
                        overflowWrap: "anywhere",
                    }}
                >
                    {tile.who}
                </Box>
            </Box>
            <ExternalIcon />
        </Box>
    </Box>
)

const FAQ = ["why", "catch", "spent", "hub"] as const

export interface IAuditCheckProps {
    /** The audited ballot's Tracking ID. */
    trackingId: string
    tenantId: string
    eventId: string
    electionId: string
    electionName?: string
    /** The check of the ballot's `audited` entry, once the log answered. */
    view?: IReceiptView
    /** Whether the audited ballot's secret is in the public record; none when
     * this portal publishes nothing. */
    publication?: EAuditPublishState
    /** Publishes it again, after a failure. */
    onPublishRetry?: () => void
    /** The election's policy: the view's, else the one the event pins. */
    policy?: Policy
    /** When the ballot box received the encrypted ballot, ISO 8601. */
    receivedAt?: string
    /** When the ballot was audited, as the voter reads it, for the dialog. */
    auditedAt?: string
    /** The election authority's own ballot verifier (`BALLOT_VERIFIER_URL`). */
    ballotVerifierUrl?: string
    /** This ballot in the portal's own ballot tracker. */
    trackerUrl: string
    /** Where Vote again goes. */
    voteAgainTo: string
    steps: React.ReactNode
    onTitleHelp: () => void
    onBallotIdHelp: () => void
}

/** The tiles are drawn in a random order when the screen appears, and kept
 * while it updates. Give the component a `key` of the policy ID so that a
 * different policy draws again. */
export const AuditCheck: React.FC<IAuditCheckProps> = ({
    trackingId,
    tenantId,
    eventId,
    electionId,
    electionName,
    view,
    publication,
    onPublishRetry,
    policy,
    receivedAt,
    auditedAt,
    ballotVerifierUrl,
    trackerUrl,
    voteAgainTo,
    steps,
    onTitleHelp,
    onBallotIdHelp,
}) => {
    const {t} = useTranslation()
    const faqId = useId()
    const [why, setWhy] = useState(false)
    const [hubs] = useState(() => shuffled(auditHubs(policy)))
    const tracker = useTrackerLink(trackerUrl)
    const fullId = groupBallotId(trackingId)
    const path = {tenantId, eventId, electionId}
    const link = (base: string) => auditedBallotLink(base, path, fullId, policy?.id)

    const tiles: IAuditTile[] =
        hubs.length > 0
            ? hubs.map((hub) => ({
                  name: hub.name,
                  who: t(`${KEYS}.runBy`, {party: hub.party}),
                  label: t(`${KEYS}.hubLabel`, {name: hub.name, party: hub.party}),
                  href: link(hub.site),
              }))
            : ballotVerifierUrl
              ? [
                    {
                        name: t(`${KEYS}.authorityName`),
                        who: t(`${KEYS}.runByAuthority`),
                        label: t(`${KEYS}.authorityLabel`),
                        href: link(ballotVerifierUrl),
                    },
                ]
              : []
    const footnote =
        hubs.length > 0 ? `${KEYS}.footnote` : tiles.length > 0 ? `${KEYS}.footnoteAuthority` : ""

    const receivedSeconds = receivedAt ? Date.parse(receivedAt) / 1000 : NaN
    const acceptedAt = view?.own?.acceptedAt
    const entry = view?.result?.entry?.index
    const opened = [
        acceptedAt === undefined ? undefined : clockMinutes(acceptedAt),
        entry === undefined ? undefined : t(`${KEYS}.entry`, {index: entry}),
    ]
        .filter(Boolean)
        .join(" · ")
    const published = publication === undefined || publication === EAuditPublishState.PUBLISHED
    const failed = publication === EAuditPublishState.FAILED

    return (
        <Box className="audit-check">
            <AuditHeader className="audit-header">
                <Box className="audit-header-steps stepper-box">{steps}</Box>
                <Box className="audit-header-title">
                    <Box
                        component="h1"
                        className="screen-title"
                        sx={{
                            display: "flex",
                            alignItems: "center",
                            gap: "16px",
                            m: 0,
                            fontSize: "1.5rem",
                            lineHeight: 1.235,
                            fontWeight: 700,
                            color: COLORS.text,
                        }}
                    >
                        <Box component="span" className="screen-title-text">
                            {t(`${KEYS}.title`)}
                        </Box>
                        <IconButton
                            buttonClassName="screen-help-button"
                            icon={faCircleQuestion}
                            sx={{fontSize: "unset", lineHeight: "unset", paddingBottom: "2px"}}
                            fontSize="1rem"
                            onClick={onTitleHelp}
                            ariaLabel={t("a11y.helpAbout", {topic: t(`${KEYS}.title`)})}
                        />
                    </Box>
                </Box>
                <Box className="audit-header-id">
                    <ReviewBallotId
                        ballotId={trackingId}
                        state={EReviewBallotIdState.AUDITED}
                        view={view}
                        auditedAt={auditedAt}
                        trackerUrl={trackerUrl}
                        electionName={electionName}
                        helpLabel={t("reviewScreen.ballotIdHelpDialog.title")}
                        onHelpClick={onBallotIdHelp}
                    />
                </Box>
            </AuditHeader>
            <Box
                component="p"
                className="screen-description audit-lead"
                sx={{
                    m: "14px 0 0",
                    maxWidth: 760,
                    fontSize: "16px",
                    lineHeight: 1.5,
                    color: COLORS.body,
                    [PHONE]: {fontSize: "15px"},
                }}
            >
                {t(`${KEYS}.lead`)}
            </Box>

            <Box
                component="ol"
                className="audit-steps"
                aria-label={t(`${KEYS}.label`)}
                sx={{
                    m: "28px 0 0",
                    p: 0,
                    listStyle: "none",
                    border: `1px solid ${COLORS.line}`,
                    borderRadius: "12px",
                    overflow: "hidden",
                }}
            >
                <Step
                    className="audit-step-encrypted"
                    number={1}
                    title={t(`${KEYS}.encrypted.title`)}
                    text={t(`${KEYS}.encrypted.text`)}
                    state={EStepState.DONE}
                    when={Number.isFinite(receivedSeconds) ? clockMinutes(receivedSeconds) : ""}
                />
                <Step
                    className="audit-step-opened"
                    number={2}
                    state={
                        published
                            ? EStepState.DONE
                            : failed
                              ? EStepState.FAILED
                              : EStepState.CURRENT
                    }
                    title={t(failed ? `${KEYS}.opened.failedTitle` : `${KEYS}.opened.title`)}
                    text={t(
                        published
                            ? `${KEYS}.opened.text`
                            : failed
                              ? `${KEYS}.opened.failedText`
                              : `${KEYS}.opened.publishingText`
                    )}
                    when={published ? opened : ""}
                >
                    {failed && onPublishRetry ? (
                        <Box className="audit-publish-retry" sx={{...underSx, mt: "12px"}}>
                            <Button
                                className="audit-publish-retry-button"
                                variant="secondary"
                                onClick={onPublishRetry}
                            >
                                {t("ballotPrivacy.retry")}
                            </Button>
                        </Box>
                    ) : null}
                </Step>
                <Step
                    className="audit-step-compare"
                    number={3}
                    state={published ? EStepState.CURRENT : EStepState.NEXT}
                    title={t(`${KEYS}.compare.title`)}
                    text={t(`${KEYS}.compare.text`)}
                >
                    {!published ? null : tiles.length > 0 ? (
                        <Box
                            component="ul"
                            className="audit-hubs"
                            aria-label={t(`${KEYS}.hubsLabel`)}
                            sx={{
                                ...underSx,
                                display: "grid",
                                gridTemplateColumns: "repeat(auto-fill, minmax(250px, 1fr))",
                                gap: "10px",
                                m: "14px 0 0",
                                p: 0,
                                listStyle: "none",
                                [PHONE]: {
                                    gridColumn: "1 / -1",
                                    gridTemplateColumns: "minmax(0, 1fr)",
                                },
                            }}
                        >
                            {tiles.map((tile) => (
                                <HubTile key={tile.name} tile={tile} />
                            ))}
                        </Box>
                    ) : (
                        <Box
                            component="p"
                            className="audit-tracker"
                            sx={{
                                ...underSx,
                                m: "12px 0 0",
                                fontSize: "14px",
                                lineHeight: "20px",
                                color: COLORS.tip,
                            }}
                        >
                            {t(`${KEYS}.trackerText`)}{" "}
                            <Box
                                component="a"
                                className="audit-tracker-link"
                                {...tracker}
                                sx={{color: COLORS.navy, fontWeight: 500}}
                            >
                                {t(`${KEYS}.trackerLink`)}
                            </Box>
                        </Box>
                    )}
                    {published && footnote ? (
                        <Box
                            component="p"
                            className="audit-hubs-foot"
                            sx={{
                                ...underSx,
                                m: "10px 0 0",
                                fontSize: "13px",
                                lineHeight: "19px",
                                color: COLORS.quiet,
                            }}
                        >
                            {t(footnote)}
                        </Box>
                    ) : null}
                    {!published ? null : (
                        <Box
                            className="audit-tip"
                            sx={{
                                ...underSx,
                                display: "flex",
                                gap: "10px",
                                alignItems: "flex-start",
                                mt: "12px",
                                fontSize: "13px",
                                lineHeight: "19px",
                                color: COLORS.tip,
                            }}
                        >
                            <PhoneIcon />
                            <span className="audit-tip-text">
                                {t(hubs.length > 0 ? `${KEYS}.tip` : `${KEYS}.tipTracker`)}
                            </span>
                        </Box>
                    )}
                </Step>
            </Box>

            <Box
                component="button"
                type="button"
                className="audit-why"
                aria-expanded={why}
                aria-controls={faqId}
                onClick={() => setWhy((open) => !open)}
                sx={{
                    "display": "inline-flex",
                    "alignItems": "center",
                    "gap": "6px",
                    "minHeight": 40,
                    "m": "18px 0 0 -6px",
                    "p": "0 6px",
                    "border": 0,
                    "borderRadius": "4px",
                    "bgcolor": "transparent",
                    "fontFamily": "inherit",
                    "fontSize": "15px",
                    "fontWeight": 500,
                    "color": COLORS.navy,
                    "cursor": "pointer",
                    "&:hover": {bgcolor: "#F1F5F9"},
                    "&:focus-visible": {outline: `2px solid ${COLORS.navy}`, outlineOffset: "2px"},
                }}
            >
                {t(`${KEYS}.why`)}
                <Chevron open={why} />
            </Box>
            {why ? (
                <Box
                    id={faqId}
                    className="audit-faq"
                    sx={{
                        display: "grid",
                        gridTemplateColumns: "repeat(2, minmax(0, 1fr))",
                        gap: "14px 28px",
                        m: "6px 0 0",
                        pt: "14px",
                        borderTop: `1px solid ${COLORS.rule}`,
                        [PHONE]: {gridTemplateColumns: "minmax(0, 1fr)"},
                    }}
                >
                    {FAQ.map((item) => (
                        <Box key={item} className={`audit-faq-item audit-faq-${item}`}>
                            <Box
                                className="audit-faq-question"
                                sx={{fontSize: "14px", lineHeight: "20px", fontWeight: 600}}
                            >
                                {t(`${KEYS}.faq.${item}.question`)}
                            </Box>
                            <Box
                                component="p"
                                className="audit-faq-answer"
                                sx={{
                                    m: "2px 0 0",
                                    fontSize: "14px",
                                    lineHeight: "21px",
                                    color: COLORS.body,
                                }}
                            >
                                {t(`${KEYS}.faq.${item}.answer`)}
                            </Box>
                        </Box>
                    ))}
                </Box>
            ) : null}

            <Box
                className="actions-container audit-actions"
                sx={{
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "flex-end",
                    m: "34px 0 24px",
                    [PHONE]: {m: "28px 0 24px"},
                }}
            >
                <Button
                    className="restart-voting-button"
                    component={RouterLink}
                    to={voteAgainTo}
                    sx={{margin: "auto 0", width: {xs: "100%", sm: "200px"}, gap: "6px"}}
                >
                    <Box component="span" className="restart-voting-label">
                        {t("auditScreen.voteAgain")}
                    </Box>
                    <Icon className="restart-voting-icon" icon={faAngleRight} size="sm" />
                </Button>
            </Box>
        </Box>
    )
}
