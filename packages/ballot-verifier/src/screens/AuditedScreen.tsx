// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The audited ballot screen (design boards HubAudit and PhoneHubAudit): a
// transparency hub's ballot verifier, opened from the voting portal's audit
// screen with the ballot's full Tracking ID and the election's policy ID in
// the fragment. It needs no login and no file: it finds the ballot in the
// public record, checks its audit there, checks that the secret the portal
// published matches the ballot, decodes it in this browser, and asks the
// voter whether the choices are theirs. The answer stays in this browser.

import React, {useContext, useMemo} from "react"
import {useLocation, useParams} from "react-router-dom"
import Box from "@mui/material/Box"
import ButtonBase from "@mui/material/ButtonBase"
import CircularProgress from "@mui/material/CircularProgress"
import {useTranslation} from "react-i18next"
import {AuditCompare} from "../components/Audited/AuditCompare"
import {DecodedContests} from "../components/Audited/DecodedContests"
import {FullTrackingId} from "../components/Audited/FullTrackingId"
import {HubHeader} from "../components/Audited/HubHeader"
import {FONT, HUB_COLORS, MONO, PALETTE, PHONE} from "../components/Audited/hubStyle"
import {SettingsContext} from "../providers/SettingsContextProvider"
import {
    auditEvidence,
    authorityContact,
    evidenceFileName,
    EVIDENCE_MEDIA_TYPE,
} from "../services/vitrine/auditEvidence"
import {EAuditOutcome} from "../services/vitrine/ballotAudit"
import {IAuditedIds, parseAuditedFragment} from "../services/vitrine/auditedBallot"
import {shortBallotId} from "../services/vitrine/ballotId"
import {saveTextFile} from "../services/vitrine/file"
import {
    EAuditedPhase,
    ECheckState,
    IAuditedBallot,
    IAuditedDeps,
    useAuditedBallot,
} from "./hooks/useAuditedBallot"

const KEYS = "auditedScreen"

/** The phases that end in a card instead of the comparison, and its tone. */
const PROBLEM_PHASES: Readonly<Partial<Record<EAuditedPhase, "info" | "problem">>> = {
    [EAuditedPhase.BAD_LINK]: "info",
    [EAuditedPhase.NO_POLICY]: "info",
    [EAuditedPhase.OTHER_ELECTION]: "info",
    [EAuditedPhase.NOT_FOUND]: "info",
    [EAuditedPhase.CAST]: "info",
    [EAuditedPhase.PENDING]: "info",
    [EAuditedPhase.NO_CONTENT]: "info",
    [EAuditedPhase.UNAVAILABLE]: "info",
    [EAuditedPhase.UNCHECKED]: "info",
    [EAuditedPhase.RECORD_FAILED]: "problem",
    [EAuditedPhase.MISMATCH]: "problem",
}

/** The phases that are worth another try: a server didn't answer, or the
 * record isn't there yet. */
const RETRY_PHASES: readonly EAuditedPhase[] = [
    EAuditedPhase.NO_POLICY,
    EAuditedPhase.NOT_FOUND,
    EAuditedPhase.PENDING,
    EAuditedPhase.NO_CONTENT,
    EAuditedPhase.UNAVAILABLE,
]

/** When the ballot box took the audit: the time today, else the date too. */
export const auditedTime = (seconds: number, language: string, now: Date = new Date()): string => {
    const at = new Date(seconds * 1000)
    const today = at.toDateString() === now.toDateString()
    try {
        return new Intl.DateTimeFormat(
            language,
            today ? {timeStyle: "short"} : {dateStyle: "medium", timeStyle: "short"}
        ).format(at)
    } catch {
        return at.toISOString()
    }
}

/** Where the voting site lets the voter vote in this election. */
const votingSiteUrl = (base: string | undefined, ids: IAuditedIds): string | undefined => {
    const url = base?.trim()
    if (!url) return undefined
    const root = url.endsWith("/") ? url : `${url}/`
    return `${root}tenant/${encodeURIComponent(ids.tenantId)}/event/${encodeURIComponent(ids.eventId)}/election/${encodeURIComponent(ids.electionId)}/start`
}

const StatusTag: React.FC<{label: string}> = ({label}) => (
    <Box
        component="span"
        className="audited-tag"
        sx={{
            display: "inline-flex",
            alignItems: "center",
            gap: "6px",
            height: "24px",
            p: "0 10px",
            border: `1px solid ${PALETTE.green}`,
            borderRadius: "999px",
            backgroundColor: "#fff",
            color: PALETTE.green,
            fontSize: "13px",
            lineHeight: 1,
            fontWeight: 600,
            whiteSpace: "nowrap",
        }}
    >
        <Box
            component="span"
            aria-hidden="true"
            sx={{width: 6, height: 6, borderRadius: "50%", backgroundColor: PALETTE.green}}
        />
        {label}
    </Box>
)

const CheckIcon: React.FC<{state: ECheckState}> = ({state}) => {
    const box = {flex: "none", marginTop: "1px"} as const
    if (state === ECheckState.WAITING) {
        return (
            <Box component="span" sx={{...box, display: "inline-flex", width: 20, height: 20}}>
                <CircularProgress size={16} thickness={5} sx={{color: PALETTE.quiet, m: "2px"}} />
            </Box>
        )
    }
    const failed = state === ECheckState.FAILED
    return (
        <svg
            className={`audited-check-icon audited-check-icon-${state}`}
            width="20"
            height="20"
            viewBox="0 0 24 24"
            fill="none"
            stroke={failed ? PALETTE.redInk : "#7D8799"}
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
            aria-hidden="true"
            style={box}
        >
            {failed ? <path d="M6 6l12 12M18 6L6 18" /> : <path d="M4.5 12.5l4.75 4.75L19.5 7" />}
        </svg>
    )
}

interface IChecksProps {
    audited: IAuditedBallot
    language: string
}

/** The aside: what this browser checked, one line each. */
const Checks: React.FC<IChecksProps> = ({audited, language}) => {
    const {t} = useTranslation()
    const {checks, entry} = audited
    const items = [
        {
            key: "record",
            state: checks.record,
            text:
                checks.record === ECheckState.DONE && entry
                    ? t(`${KEYS}.checks.record.text`, {
                          index: entry.index,
                          time: auditedTime(entry.acceptedAt, language),
                      })
                    : t(`${KEYS}.checks.record.${checks.record}`),
        },
        {
            key: "secret",
            state: checks.secret,
            text: t(
                `${KEYS}.checks.secret.${checks.secret === ECheckState.DONE ? "text" : checks.secret}`
            ),
        },
        {
            key: "decoded",
            state: checks.decoded,
            text: t(
                `${KEYS}.checks.decoded.${checks.decoded === ECheckState.DONE ? "text" : checks.decoded}`
            ),
        },
    ]
    return (
        <Box
            component="section"
            className="audited-checks"
            aria-labelledby="audited-checks-title"
            sx={{
                p: "18px",
                border: `1px solid ${PALETTE.border}`,
                borderRadius: "12px",
                backgroundColor: "#fff",
            }}
        >
            <Box
                component="h2"
                id="audited-checks-title"
                sx={{m: 0, fontSize: 16, lineHeight: "22px", fontWeight: 700}}
            >
                {t(`${KEYS}.checks.title`)}
            </Box>
            <Box
                component="ul"
                sx={{display: "grid", gap: "12px", m: "12px 0 0", p: 0, listStyle: "none"}}
            >
                {items.map((item) => (
                    <Box
                        component="li"
                        key={item.key}
                        className={`audited-check audited-check-${item.key}`}
                        data-state={item.state}
                        sx={{display: "flex", gap: "10px", alignItems: "flex-start"}}
                    >
                        <CheckIcon state={item.state} />
                        <Box>
                            <Box sx={{fontSize: 14, lineHeight: "20px", fontWeight: 600}}>
                                {t(`${KEYS}.checks.${item.key}.title`)}
                            </Box>
                            <Box sx={{fontSize: 13, lineHeight: "18px", color: PALETTE.quiet}}>
                                {item.text}
                            </Box>
                        </Box>
                    </Box>
                ))}
            </Box>
        </Box>
    )
}

const LockNote: React.FC<{hub?: string}> = ({hub}) => {
    const {t} = useTranslation()
    return (
        <Box
            component="p"
            className="audited-note"
            sx={{
                display: "flex",
                gap: "8px",
                m: 0,
                fontSize: 13,
                lineHeight: "18px",
                color: PALETTE.quiet,
            }}
        >
            <svg
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
                aria-hidden="true"
                style={{flex: "none", marginTop: "1px"}}
            >
                <rect x="5" y="11" width="14" height="10" rx="2" />
                <path d="M8 11V8a4 4 0 0 1 8 0v3" />
            </svg>
            <span>{t(`${KEYS}.note.${hub ? "named" : "unnamed"}`, {hub})}</span>
        </Box>
    )
}

interface IStateCardProps {
    phase: EAuditedPhase
    tone: "info" | "problem"
    onRetry?: () => void
}

/** Why there is nothing to compare. */
const StateCard: React.FC<IStateCardProps> = ({phase, tone, onRetry}) => {
    const {t} = useTranslation()
    const problem = tone === "problem"
    return (
        <Box
            component="section"
            role={problem ? "alert" : "status"}
            className={`audited-state audited-state-${phase}`}
            data-testid="audited-state"
            sx={{
                p: "24px",
                border: `1px solid ${problem ? "#FECACA" : PALETTE.border}`,
                borderRadius: "4px",
                backgroundColor: problem ? "#FEF2F2" : PALETTE.tint,
                color: problem ? "#7F1D1D" : PALETTE.text,
                [PHONE]: {p: "20px 16px"},
            }}
        >
            <Box
                component="h2"
                className="audited-state-title"
                sx={{
                    m: 0,
                    fontSize: 20,
                    lineHeight: "28px",
                    fontWeight: 700,
                    color: problem ? PALETTE.redInk : PALETTE.text,
                }}
            >
                {t(`${KEYS}.states.${phase}.title`)}
            </Box>
            <Box
                component="p"
                className="audited-state-text"
                sx={{
                    m: "8px 0 0",
                    fontSize: 14,
                    lineHeight: "20px",
                    color: problem ? "inherit" : PALETTE.muted,
                }}
            >
                {t(`${KEYS}.states.${phase}.text`)}
            </Box>
            {onRetry ? (
                <ButtonBase
                    className="audited-state-retry"
                    onClick={onRetry}
                    sx={{
                        "mt": "16px",
                        "minHeight": 44,
                        "p": "0 16px",
                        "border": `1px solid ${PALETTE.navy}`,
                        "borderRadius": "4px",
                        "backgroundColor": "#fff",
                        "fontSize": 15,
                        "fontWeight": 500,
                        "color": PALETTE.navy,
                        "&.Mui-focusVisible": {
                            outline: `2px solid ${PALETTE.navy}`,
                            outlineOffset: "2px",
                        },
                    }}
                >
                    {t(`${KEYS}.retry`)}
                </ButtonBase>
            ) : null}
        </Box>
    )
}

const Loading: React.FC = () => {
    const {t} = useTranslation()
    return (
        <Box
            role="status"
            className="audited-loading"
            sx={{
                display: "flex",
                alignItems: "center",
                gap: "12px",
                p: "24px 0",
                color: PALETTE.muted,
            }}
        >
            <CircularProgress size={20} sx={{color: HUB_COLORS.key}} />
            <span>{t(`${KEYS}.loading`)}</span>
        </Box>
    )
}

export interface IAuditedScreenProps {
    /** What the screen uses from the outside, for tests. */
    deps?: Partial<IAuditedDeps>
}

export const AuditedScreen: React.FC<IAuditedScreenProps> = ({deps}) => {
    const {t, i18n} = useTranslation()
    const {globalSettings} = useContext(SettingsContext)
    const {tenantId = "", eventId = "", electionId = ""} = useParams()
    const {hash} = useLocation()
    const link = useMemo(() => parseAuditedFragment(hash), [hash])
    const ids = useMemo(() => ({tenantId, eventId, electionId}), [tenantId, eventId, electionId])
    const audited = useAuditedBallot(
        {
            ids,
            link,
            publicBucketUrl: globalSettings.PUBLIC_BUCKET_URL,
            graphqlUrl: globalSettings.HASURA_URL,
            ...(globalSettings.HUB_NAME ? {hubName: globalSettings.HUB_NAME} : {}),
        },
        deps
    )
    const language = i18n.resolvedLanguage || i18n.language || "en"
    const {phase, record, audit, view} = audited
    const tone = PROBLEM_PHASES[phase]
    const verified = audit?.outcome === EAuditOutcome.VERIFIED ? audit : undefined
    const tagged =
        phase === EAuditedPhase.READY ||
        phase === EAuditedPhase.MISMATCH ||
        phase === EAuditedPhase.NO_CONTENT ||
        phase === EAuditedPhase.UNCHECKED

    let body: React.ReactNode
    if (phase === EAuditedPhase.READY && record && verified && link) {
        const contact = authorityContact(view?.policy)
        const saveEvidence = () =>
            saveTextFile(
                evidenceFileName(link.trackingId),
                auditEvidence({
                    ballotId: link.trackingId,
                    record,
                    selections: verified.selections,
                }),
                EVIDENCE_MEDIA_TYPE
            )
        body = (
            <AuditCompare
                ballotId={link.trackingId}
                backHref={votingSiteUrl(globalSettings.VOTING_PORTAL_URL, ids)}
                onSaveEvidence={saveEvidence}
                {...(contact ? {contact} : {})}
            >
                <DecodedContests
                    ballotStyle={record.ballot.config}
                    selections={verified.selections}
                    language={language}
                    publicBucketUrl={globalSettings.PUBLIC_BUCKET_URL}
                />
            </AuditCompare>
        )
    } else if (tone) {
        body = (
            <StateCard
                phase={phase}
                tone={tone}
                {...(RETRY_PHASES.includes(phase) ? {onRetry: audited.retry} : {})}
            />
        )
    } else {
        body = <Loading />
    }

    return (
        <Box
            className="audited-screen"
            sx={{width: "100%", fontFamily: FONT, color: PALETTE.text, backgroundColor: "#fff"}}
        >
            <HubHeader hub={audited.hub} />
            <Box
                component="main"
                id="main-content"
                tabIndex={-1}
                className="audited-main"
                sx={{
                    boxSizing: "border-box",
                    maxWidth: "1184px",
                    m: "0 auto",
                    p: "24px 24px 48px",
                    display: "grid",
                    gridTemplateColumns: "minmax(0, 1fr) 300px",
                    gap: "32px",
                    alignItems: "start",
                    [PHONE]: {
                        maxWidth: "none",
                        p: "16px 16px 32px",
                        gridTemplateColumns: "minmax(0, 1fr)",
                        gap: "20px",
                    },
                }}
            >
                <Box sx={{minWidth: 0}}>
                    <Box
                        sx={{
                            display: "flex",
                            alignItems: "center",
                            gap: "10px",
                            flexWrap: "wrap",
                            m: "8px 0 14px",
                        }}
                    >
                        <Box
                            component="h1"
                            className="audited-title"
                            sx={{
                                m: 0,
                                fontSize: 28,
                                lineHeight: 1.25,
                                fontWeight: 700,
                                [PHONE]: {fontSize: 22},
                            }}
                        >
                            {t(`${KEYS}.title`)}
                            {link ? (
                                <>
                                    {" "}
                                    <Box component="span" sx={{fontFamily: MONO}}>
                                        {shortBallotId(link.trackingId)}
                                    </Box>
                                </>
                            ) : null}
                        </Box>
                        {tagged ? <StatusTag label={t(`${KEYS}.tag`)} /> : null}
                    </Box>
                    {link ? <FullTrackingId ballotId={link.trackingId} /> : null}
                    <Box sx={{mt: "20px"}}>{body}</Box>
                </Box>
                <Box
                    component="aside"
                    sx={{display: "flex", flexDirection: "column", gap: "16px", minWidth: 0}}
                >
                    <Checks audited={audited} language={language} />
                    <LockNote {...(audited.hub.name ? {hub: audited.hub.name} : {})} />
                </Box>
            </Box>
        </Box>
    )
}
