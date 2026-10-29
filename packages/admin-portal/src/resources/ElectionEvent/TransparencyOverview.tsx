// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The overview at the top of the Transparency tab, laid out as Vitrine's
// admin design board: the record's health, its figures, the policy's
// witnesses and hubs, the milestones, and the event's visibility. A figure
// Harvest doesn't report says so.

import React, {useId, useState} from "react"
import {useTranslation} from "react-i18next"
import {
    Alert,
    AlertTitle,
    Box,
    Button,
    Dialog,
    DialogActions,
    DialogContent,
    DialogContentText,
    DialogTitle,
    Link,
    Typography,
} from "@mui/material"
import type {SxProps, Theme} from "@mui/material"
import {ENABLED_MODES, ETransparencyMode, isKnownRefusal} from "@/services/Transparency"
import {
    EHealth,
    EWitnessState,
    IHealth,
    IHubRow,
    IPolicyFacts,
    IRecordFigures,
    IWitnessRow,
} from "@/services/TransparencyOverview"

export const BOARD = {
    font: "'Helvetica Neue', Helvetica, Arial, sans-serif",
    navy: "#0F054C",
    green: "#047857",
    warning: "#C2410C",
    text: "#191D23",
    body: "#334155",
    muted: "#475569",
    subtle: "#64748B",
    border: "#E7EAEE",
    tint: "#F7F9FE",
    white: "#FFFFFF",
} as const

const focusRing = {outline: `2px solid ${BOARD.navy}`, outlineOffset: "2px"}

const buttonBase = {
    "minHeight": "44px",
    "px": 2,
    "borderRadius": 0,
    "fontFamily": "inherit",
    "fontSize": "14px",
    "fontWeight": 600,
    "textTransform": "none",
    "boxShadow": "none",
    "&.Mui-focusVisible, &:focus-visible": focusRing,
    "&.Mui-disabled": {bgcolor: BOARD.border, color: BOARD.subtle, borderColor: BOARD.border},
} as const

// The admin theme fills every button with its brand colour, and flips the
// colours of a button that is focused or pressed: each state says its own.
export const filledButton: SxProps<Theme> = {
    ...buttonBase,
    "bgcolor": BOARD.navy,
    "color": BOARD.white,
    "border": `1px solid ${BOARD.navy}`,
    "&:hover, &:focus": {
        bgcolor: BOARD.navy,
        color: BOARD.white,
        border: `1px solid ${BOARD.navy}`,
    },
    "&:active": {bgcolor: `${BOARD.navy} !important`, color: `${BOARD.white} !important`},
}

export const outlinedButton: SxProps<Theme> = {
    ...buttonBase,
    "bgcolor": BOARD.white,
    "color": BOARD.navy,
    "border": `1px solid ${BOARD.navy}`,
    "&:hover, &:focus": {
        bgcolor: BOARD.white,
        color: BOARD.navy,
        border: `1px solid ${BOARD.navy}`,
    },
    "&:active": {bgcolor: `${BOARD.white} !important`, color: `${BOARD.navy} !important`},
}

/** An inline action beside a value, such as "Copy". */
export const textButton: SxProps<Theme> = {
    "minHeight": "44px",
    "px": 1.5,
    "borderRadius": 0,
    "border": "1px solid transparent",
    "fontFamily": "inherit",
    "fontSize": "13.5px",
    "fontWeight": 600,
    "textTransform": "none",
    "bgcolor": "transparent",
    "color": BOARD.navy,
    "boxShadow": "none",
    "&:hover, &:focus": {
        bgcolor: "transparent",
        color: BOARD.navy,
        border: "1px solid transparent",
        boxShadow: "none",
        textDecoration: "underline",
    },
    "&:active": {bgcolor: "transparent !important", color: `${BOARD.navy} !important`},
    "&.Mui-focusVisible, &:focus-visible": focusRing,
}

const linkStyle = {
    "color": BOARD.navy,
    "fontWeight": 600,
    "fontSize": "14px",
    "minHeight": "44px",
    "display": "inline-flex",
    "alignItems": "center",
    "&:focus-visible": focusRing,
} as const

/** A grid of at most `max` equal columns, each at least `min` pixels wide:
 * fewer columns when the tab is narrower, whatever the screen. */
export const columns = (max: number, min: number, gap: number): string =>
    `repeat(auto-fit, minmax(min(100%, max(${min}px, calc((100% - ${(max - 1) * gap}px) / ${max}))), 1fr))`

// -- Building blocks ----------------------------------------------------------------

export interface IBoardSectionProps {
    title: string
    /** A note to the right of the title. */
    aside?: React.ReactNode
    testId?: string
    /** True when the children run edge to edge, as list rows do. */
    flush?: boolean
    children?: React.ReactNode
}

export const BoardSection: React.FC<IBoardSectionProps> = ({
    title,
    aside,
    testId,
    flush,
    children,
}) => {
    const id = useId()
    return (
        <Box
            component="section"
            aria-labelledby={id}
            data-testid={testId}
            sx={{
                bgcolor: BOARD.white,
                border: `1px solid ${BOARD.border}`,
                minWidth: 0,
                display: "flex",
                flexDirection: "column",
            }}
        >
            <Box
                sx={{
                    display: "flex",
                    flexWrap: "wrap",
                    alignItems: "baseline",
                    justifyContent: "space-between",
                    gap: "4px 12px",
                    p: flush ? "16px 18px 12px" : "16px 18px 0",
                }}
            >
                <Typography
                    component="h3"
                    id={id}
                    sx={{m: 0, fontSize: "16px", fontWeight: 700, color: BOARD.text}}
                >
                    {title}
                </Typography>
                {aside ? (
                    <Box
                        component="span"
                        sx={{
                            display: "flex",
                            alignItems: "center",
                            gap: "6px",
                            fontSize: "12.5px",
                            color: BOARD.subtle,
                        }}
                    >
                        {aside}
                    </Box>
                ) : null}
            </Box>
            {flush ? (
                children
            ) : (
                <Box
                    sx={{
                        p: "12px 18px 18px",
                        display: "flex",
                        flexDirection: "column",
                        gap: "12px",
                        fontSize: "13.5px",
                        lineHeight: 1.5,
                        color: BOARD.muted,
                    }}
                >
                    {children}
                </Box>
            )}
        </Box>
    )
}

export enum ETone {
    OK = "ok",
    WARN = "warn",
    ALERT = "alert",
    NEUTRAL = "neutral",
}

const TONES: Record<ETone, {bg: string; fg: string}> = {
    [ETone.OK]: {bg: "#CFF0DC", fg: "#064E3B"},
    [ETone.WARN]: {bg: "#FFF7D9", fg: "#6B5A1F"},
    [ETone.ALERT]: {bg: "#FFEDD5", fg: "#9A3412"},
    [ETone.NEUTRAL]: {bg: "#F1F5F9", fg: BOARD.muted},
}

export const StatusChip: React.FC<{tone: ETone; label: string; testId?: string}> = ({
    tone,
    label,
    testId,
}) => (
    <Box
        component="span"
        data-testid={testId}
        data-tone={tone}
        sx={{
            flex: "none",
            fontSize: "12px",
            fontWeight: 600,
            lineHeight: 1.3,
            p: "4px 8px",
            bgcolor: TONES[tone].bg,
            color: TONES[tone].fg,
            whiteSpace: "nowrap",
        }}
    >
        {label}
    </Box>
)

const rowSx = {
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    gap: "12px",
    p: "11px 18px",
    borderTop: `1px solid ${BOARD.border}`,
    minWidth: 0,
} as const

const Row: React.FC<{name: string; sub?: string; chip: React.ReactNode; testId: string}> = ({
    name,
    sub,
    chip,
    testId,
}) => (
    <Box component="li" sx={rowSx} data-testid={testId}>
        <Box sx={{display: "flex", flexDirection: "column", gap: "2px", minWidth: 0}}>
            <Box
                component="span"
                sx={{fontSize: "14px", color: BOARD.text, overflowWrap: "anywhere"}}
            >
                {name}
            </Box>
            {sub ? (
                <Box
                    component="span"
                    sx={{fontSize: "12.5px", color: BOARD.subtle, overflowWrap: "anywhere"}}
                >
                    {sub}
                </Box>
            ) : null}
        </Box>
        {chip}
    </Box>
)

const Rows: React.FC<{children: React.ReactNode}> = ({children}) => (
    <Box component="ul" sx={{listStyle: "none", m: 0, p: 0}}>
        {children}
    </Box>
)

const LockIcon: React.FC = () => (
    <svg
        width="14"
        height="14"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
    >
        <rect x="5" y="10.5" width="14" height="10" />
        <path d="M8 10.5V7.5a4 4 0 0 1 8 0v3" />
    </svg>
)

const HealthIcon: React.FC<{health: EHealth}> = ({health}) => {
    if (health === EHealth.HEALTHY) {
        return (
            <svg
                width="40"
                height="40"
                viewBox="0 0 40 40"
                aria-hidden="true"
                style={{flex: "none"}}
            >
                <circle cx="20" cy="20" r="18" fill="none" stroke={BOARD.green} strokeWidth="2.5" />
                <circle
                    cx="20"
                    cy="20"
                    r="13.5"
                    fill="none"
                    stroke={BOARD.navy}
                    strokeWidth="2.5"
                />
                <path
                    d="M14.5 20.5l3.6 3.6 7.4-7.6"
                    fill="none"
                    stroke={BOARD.navy}
                    strokeWidth="2.6"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                />
            </svg>
        )
    }
    const warning = health === EHealth.ATTENTION || health === EHealth.UNCHECKED
    const stroke = warning ? BOARD.warning : BOARD.subtle
    return (
        <svg width="40" height="40" viewBox="0 0 40 40" aria-hidden="true" style={{flex: "none"}}>
            <circle
                cx="20"
                cy="20"
                r="18"
                fill={warning ? "#FFF7ED" : BOARD.tint}
                stroke={stroke}
                strokeWidth="2.5"
            />
            {warning ? (
                <>
                    <path d="M20 11v11" stroke={stroke} strokeWidth="2.8" strokeLinecap="round" />
                    <circle cx="20" cy="28" r="1.8" fill={stroke} />
                </>
            ) : (
                <path d="M13 20h14" stroke={stroke} strokeWidth="2.6" strokeLinecap="round" />
            )}
        </svg>
    )
}

// -- Health ---------------------------------------------------------------------------

export interface IHealthBannerProps {
    health: IHealth
    mode: ETransparencyMode
    facts?: IPolicyFacts
    locked: boolean
    explorer?: string
    onDownloadPolicy?: () => void
    duration: (seconds: number) => string
}

const healthTitle = (health: EHealth, mode: ETransparencyMode): string => {
    const record = mode === ETransparencyMode.PRIVATE ? "Private" : "Public"
    switch (health) {
        case EHealth.HEALTHY:
            return `healthy${record}`
        case EHealth.ATTENTION:
            return `attention${record}`
        default:
            return health
    }
}

export const HealthBanner: React.FC<IHealthBannerProps> = ({
    health,
    mode,
    facts,
    locked,
    explorer,
    onDownloadPolicy,
    duration,
}) => {
    const {t} = useTranslation()
    const id = useId()
    const summary = (): string => {
        switch (health.health) {
            case EHealth.DISABLED:
                return t("transparencyTab.help")
            case EHealth.CHECKING:
                return t("transparencyTab.sdkLoading")
            case EHealth.UNCHECKED:
                return t("transparencyTab.sdkError")
            case EHealth.NO_POLICY:
                return t("transparencyTab.overview.health.noPolicyHelp")
            default:
                break
        }
        if (!facts) return ""
        const parts = [
            t(`transparencyTab.overview.health.${facts.visibility}`),
            t("transparencyTab.overview.health.quorum", {k: facts.quorum.k, n: facts.quorum.n}),
            t("transparencyTab.overview.health.hubs", {
                count: facts.appHubs,
                independent: facts.independentHubs,
            }),
            t("transparencyTab.overview.health.mmd", {duration: duration(facts.mmd)}),
        ]
        if (locked) parts.push(t("transparencyTab.overview.health.locked"))
        return parts.join(" · ")
    }
    const text = summary()

    return (
        <Box
            component="section"
            aria-labelledby={id}
            data-testid="transparency-health"
            data-health={health.health}
            sx={{
                bgcolor: BOARD.white,
                border: `1px solid ${BOARD.border}`,
                p: "20px",
                display: "flex",
                flexWrap: "wrap",
                alignItems: "center",
                justifyContent: "space-between",
                gap: "14px 24px",
            }}
        >
            <Box sx={{display: "flex", gap: "14px", alignItems: "center", minWidth: 0}}>
                <HealthIcon health={health.health} />
                <Box sx={{display: "flex", flexDirection: "column", gap: "4px", minWidth: 0}}>
                    <Typography
                        component="h2"
                        id={id}
                        sx={{m: 0, fontSize: "18px", fontWeight: 700, color: BOARD.text}}
                    >
                        {t(
                            `transparencyTab.overview.health.title.${healthTitle(health.health, mode)}`
                        )}
                    </Typography>
                    {text ? (
                        <Typography
                            data-testid="transparency-health-summary"
                            sx={{fontSize: "13.5px", lineHeight: 1.5, color: BOARD.muted}}
                        >
                            {text}
                        </Typography>
                    ) : null}
                    {health.issues.length > 0 ? (
                        <Box
                            component="ul"
                            data-testid="transparency-health-issues"
                            sx={{
                                m: 0,
                                p: 0,
                                listStyle: "none",
                                fontSize: "13.5px",
                                color: "#9A3412",
                            }}
                        >
                            {health.issues.map((issue, i) => (
                                <li key={`${issue.issue}-${issue.log ?? ""}-${issue.witness ?? i}`}>
                                    {t(`transparencyTab.overview.issues.${issue.issue}`, {
                                        log: issue.log,
                                        witness: issue.witness,
                                        alert: issue.alert,
                                    })}
                                </li>
                            ))}
                        </Box>
                    ) : null}
                </Box>
            </Box>
            {explorer || onDownloadPolicy ? (
                <Box sx={{display: "flex", flexWrap: "wrap", gap: "10px"}}>
                    {explorer ? (
                        <Button
                            component="a"
                            href={explorer}
                            target="_blank"
                            rel="noopener noreferrer"
                            data-testid="explorer-link"
                            sx={outlinedButton}
                        >
                            {mode === ETransparencyMode.PRIVATE
                                ? t("transparencyTab.overview.health.explorerPrivate")
                                : t("transparencyTab.overview.health.explorerPublic")}
                        </Button>
                    ) : null}
                    {onDownloadPolicy ? (
                        <Button
                            onClick={onDownloadPolicy}
                            data-testid="download-policy"
                            sx={filledButton}
                        >
                            {t("transparencyTab.overview.health.download")}
                        </Button>
                    ) : null}
                </Box>
            ) : null}
        </Box>
    )
}

// -- Figures ----------------------------------------------------------------------------

export enum EFigure {
    ACCEPTED = "accepted",
    RECORD = "record",
    SEQUENCER_LAG = "sequencerLag",
    RECEIPT_TIME = "receiptTime",
    HUB_ANCHOR = "hubAnchor",
    BROWSER_LAYER = "browserLayer",
    OVERDUE_PROMISES = "overduePromises",
    EVIDENCE_REPORTS = "evidenceReports",
}

interface IFigure {
    figure: EFigure
    /** Absent when Harvest doesn't report it. */
    value?: string
    sub: string
}

export interface IFiguresProps {
    figures: IRecordFigures
    facts?: IPolicyFacts
    reported: boolean
    now: number
    lang: string
    duration: (seconds: number) => string
}

export const Figures: React.FC<IFiguresProps> = ({
    figures,
    facts,
    reported,
    now,
    lang,
    duration,
}) => {
    const {t} = useTranslation()
    const key = (k: string) => `transparencyTab.overview.figures.${k}`
    const unreported = t(key("unreportedSub"))
    const notMeasured = t(key("notMeasured"))
    const lag =
        figures.queueOldestSeconds !== undefined
            ? duration(figures.queueOldestSeconds)
            : figures.queueRows === 0
              ? duration(0)
              : undefined
    const hubsSub = !facts
        ? unreported
        : facts.appHubs === 0
          ? t(key("noHubs"))
          : t(key("hubAnchorSub"), {
                anchored: reported ? figures.anchoredHubs : 0,
                count: facts.appHubs,
                period: duration(facts.anchorPeriod),
            })
    const list: IFigure[] = [
        {figure: EFigure.ACCEPTED, sub: notMeasured},
        {
            figure: EFigure.RECORD,
            value: figures.entries?.toLocaleString(lang),
            sub:
                figures.entries !== undefined
                    ? t(key("recordSub"), {count: figures.witnessedLogs})
                    : reported
                      ? t(key("recordNone"))
                      : unreported,
        },
        {
            figure: EFigure.SEQUENCER_LAG,
            value: lag,
            sub:
                figures.queueRows !== undefined
                    ? t(key("sequencerLagSub"), {count: figures.queueRows})
                    : unreported,
        },
        {figure: EFigure.RECEIPT_TIME, sub: notMeasured},
        {
            figure: EFigure.HUB_ANCHOR,
            value:
                figures.lastAnchor !== undefined
                    ? t("transparencyTab.overview.ago", {
                          duration: duration(now - figures.lastAnchor),
                      })
                    : undefined,
            sub: hubsSub,
        },
        {
            figure: EFigure.BROWSER_LAYER,
            sub:
                facts && facts.bootstrapHubs.length > 0
                    ? t(key("browserLayerSub"), {hubs: facts.bootstrapHubs.join(", ")})
                    : t(key("browserLayerNone")),
        },
        {
            figure: EFigure.OVERDUE_PROMISES,
            sub: facts ? t(key("mergeDelay"), {duration: duration(facts.mmd)}) : unreported,
        },
        {figure: EFigure.EVIDENCE_REPORTS, sub: t(key("noIntake"))},
    ]

    return (
        <Box
            component="dl"
            aria-label={t(key("title"))}
            sx={{
                m: 0,
                display: "grid",
                gridTemplateColumns: columns(4, 150, 12),
                gap: "12px",
            }}
        >
            {list.map(({figure, value, sub}) => (
                <Box
                    key={figure}
                    data-testid={`figure-${figure}`}
                    sx={{
                        bgcolor: BOARD.white,
                        border: `1px solid ${BOARD.border}`,
                        p: "16px",
                        display: "flex",
                        flexDirection: "column",
                        gap: "6px",
                        minWidth: 0,
                    }}
                >
                    <Box component="dt" sx={{fontSize: "12.5px", color: BOARD.subtle}}>
                        {t(key(figure))}
                    </Box>
                    <Box
                        component="dd"
                        data-testid={`figure-${figure}-value`}
                        sx={
                            value !== undefined
                                ? {
                                      m: 0,
                                      fontSize: "26px",
                                      lineHeight: "31px",
                                      fontWeight: 700,
                                      color: BOARD.navy,
                                      fontVariantNumeric: "tabular-nums",
                                  }
                                : {
                                      m: 0,
                                      fontSize: "18px",
                                      lineHeight: "31px",
                                      fontWeight: 600,
                                      color: BOARD.muted,
                                  }
                        }
                    >
                        {value ?? t("transparencyTab.overview.notReported")}
                    </Box>
                    <Box component="dd" sx={{m: 0, fontSize: "12.5px", color: BOARD.muted}}>
                        {sub}
                    </Box>
                </Box>
            ))}
        </Box>
    )
}

// -- Witnesses, hubs and the booth ---------------------------------------------------

export const WitnessesCard: React.FC<{
    rows: IWitnessRow[]
    duration: (seconds: number) => string
}> = ({rows, duration}) => {
    const {t} = useTranslation()
    const key = (k: string) => `transparencyTab.overview.witnesses.${k}`
    const chipOf = (row: IWitnessRow): {tone: ETone; label: string} => {
        switch (row.state) {
            case EWitnessState.SIGNING:
                return {tone: ETone.OK, label: t(key("signing"))}
            case EWitnessState.LAGGING:
                return {
                    tone: ETone.WARN,
                    label:
                        row.lagSeconds !== undefined
                            ? t(key("lagging"), {duration: duration(row.lagSeconds)})
                            : t(key("laggingUnknown")),
                }
            case EWitnessState.ALERT:
                return {tone: ETone.ALERT, label: t(key("alert"))}
            default:
                return {tone: ETone.NEUTRAL, label: t("transparencyTab.overview.notReported")}
        }
    }
    return (
        <BoardSection
            title={t("transparencyTab.overview.witnesses.title")}
            aside={t("transparencyTab.overview.witnesses.sub")}
            testId="overview-witnesses"
            flush
        >
            {rows.length === 0 ? (
                <Typography sx={{p: "0 18px 18px", fontSize: "13.5px", color: BOARD.muted}}>
                    {t("transparencyTab.logs.noWitnesses")}
                </Typography>
            ) : (
                <Rows>
                    {rows.map((row) => {
                        const chip = chipOf(row)
                        return (
                            <Row
                                key={row.name}
                                testId={`overview-witness-${row.name}`}
                                name={row.name}
                                sub={
                                    row.party
                                        ? t("transparencyTab.overview.runBy", {party: row.party})
                                        : undefined
                                }
                                chip={<StatusChip tone={chip.tone} label={chip.label} />}
                            />
                        )
                    })}
                </Rows>
            )}
        </BoardSection>
    )
}

export const AnchoringCard: React.FC<{
    hubs: IHubRow[]
    facts: IPolicyFacts
    reported: boolean
    now: number
    duration: (seconds: number) => string
}> = ({hubs, facts, reported, now, duration}) => {
    const {t} = useTranslation()
    const key = (k: string) => `transparencyTab.overview.anchoring.${k}`
    return (
        <BoardSection
            title={t(key("title"))}
            aside={t(key("sub"), {
                hubs: duration(facts.anchorPeriod),
                browsers: duration(facts.browserPeriod),
            })}
            testId="overview-anchoring"
            flush
        >
            <Rows>
                {hubs.map((hub) => (
                    <Row
                        key={hub.name}
                        testId={`overview-hub-${hub.name}`}
                        name={hub.name}
                        sub={
                            hub.independent
                                ? t(key("independent"), {
                                      party: hub.operator,
                                      operator: facts.operator,
                                  })
                                : t("transparencyTab.overview.runBy", {party: hub.operator})
                        }
                        chip={
                            hub.lastAnchor !== undefined ? (
                                <StatusChip
                                    tone={ETone.OK}
                                    label={t("transparencyTab.overview.ago", {
                                        duration: duration(now - hub.lastAnchor),
                                    })}
                                />
                            ) : (
                                <StatusChip
                                    tone={ETone.NEUTRAL}
                                    label={
                                        reported
                                            ? t(key("noAnchor"))
                                            : t("transparencyTab.overview.notReported")
                                    }
                                />
                            )
                        }
                    />
                ))}
                <Row
                    testId="overview-browser-layer"
                    name={t(key("browserLayer"))}
                    sub={
                        facts.bootstrapHubs.length > 0
                            ? t(key("browserLayerSub"), {hubs: facts.bootstrapHubs.join(", ")})
                            : t("transparencyTab.overview.figures.browserLayerNone")
                    }
                    chip={
                        <StatusChip
                            tone={ETone.NEUTRAL}
                            label={t("transparencyTab.overview.notReported")}
                        />
                    }
                />
            </Rows>
        </BoardSection>
    )
}

export const BoothCard: React.FC<{explorer?: string}> = ({explorer}) => {
    const {t} = useTranslation()
    const key = (k: string) => `transparencyTab.overview.booth.${k}`
    return (
        <BoardSection title={t(key("title"))} testId="overview-booth">
            <Typography sx={{fontSize: "15px", fontWeight: 600, color: BOARD.text}}>
                {t(key("release"))}
            </Typography>
            <Typography sx={{fontSize: "13.5px", lineHeight: 1.5, color: BOARD.muted}}>
                {t(key("help"))}
            </Typography>
            <Box
                sx={{
                    bgcolor: BOARD.tint,
                    p: "12px",
                    fontSize: "13px",
                    lineHeight: 1.5,
                    color: BOARD.body,
                }}
            >
                {t(key("note"))}
            </Box>
            {explorer ? (
                <Link href={explorer} target="_blank" rel="noopener noreferrer" sx={linkStyle}>
                    {t(key("explorer"))}
                </Link>
            ) : null}
        </BoardSection>
    )
}

// -- Milestones -----------------------------------------------------------------------

const MILESTONES = ["configuration", "opened", "anchored", "closed", "tally", "results"] as const

export const MilestonesCard: React.FC<{
    facts: IPolicyFacts
    duration: (seconds: number) => string
}> = ({facts, duration}) => {
    const {t} = useTranslation()
    const key = (k: string) => `transparencyTab.overview.milestones.${k}`
    return (
        <BoardSection title={t(key("title"))} testId="overview-milestones">
            <Typography sx={{fontSize: "13px", color: BOARD.muted}}>
                {t(key("unreported"))}
            </Typography>
            <Box
                component="ol"
                sx={{
                    listStyle: "none",
                    m: 0,
                    p: 0,
                    display: "grid",
                    gridTemplateColumns: columns(6, 140, 12),
                    gap: "12px",
                }}
            >
                {MILESTONES.map((m) => (
                    <Box
                        component="li"
                        key={m}
                        data-testid={`milestone-${m}`}
                        sx={{display: "flex", flexDirection: "column", gap: "6px", minWidth: 0}}
                    >
                        <Box
                            aria-hidden="true"
                            sx={{
                                height: "4px",
                                background: `repeating-linear-gradient(90deg, #94A3B8 0 6px, transparent 6px 10px)`,
                            }}
                        />
                        <Box
                            component="span"
                            sx={{fontSize: "13.5px", fontWeight: 600, color: BOARD.text}}
                        >
                            {t(key(m))}
                        </Box>
                        <Box component="span" sx={{fontSize: "12.5px", color: BOARD.muted}}>
                            {t(key(`${m}Sub`), {
                                hubs: duration(facts.anchorPeriod),
                                browsers: duration(facts.browserPeriod),
                            })}
                        </Box>
                    </Box>
                ))}
            </Box>
        </BoardSection>
    )
}

// -- Visibility -------------------------------------------------------------------------

const RefusalAlert: React.FC<{code: string}> = ({code}) => {
    const {t} = useTranslation()
    return (
        <Alert
            severity="warning"
            data-testid="public-refusal"
            sx={{borderRadius: 0, borderLeft: `3px solid ${BOARD.warning}`}}
        >
            <AlertTitle>{t("transparencyTab.mode.refusalTitle")}</AlertTitle>
            {isKnownRefusal(code)
                ? t(`transparencyTab.mode.refusals.${code}`)
                : t("transparencyTab.mode.refusalUnknown", {code})}{" "}
            {t("transparencyTab.mode.refusalOptions")}
        </Alert>
    )
}

interface IVisibilityChoiceProps {
    value?: ETransparencyMode
    disabled: boolean
    publicRefused: boolean
    onChange?: (mode: ETransparencyMode) => void
}

const VisibilityChoice: React.FC<IVisibilityChoiceProps> = ({
    value,
    disabled,
    publicRefused,
    onChange,
}) => {
    const {t} = useTranslation()
    const name = useId()
    return (
        <Box
            role="radiogroup"
            aria-label={t("transparencyTab.overview.visibility.group")}
            sx={{
                display: "grid",
                gridTemplateColumns: "repeat(2, minmax(0, 1fr))",
                border: `1px solid ${BOARD.navy}`,
            }}
        >
            {ENABLED_MODES.map((m) => {
                const checked = value === m
                const off = disabled || (m === ETransparencyMode.PUBLIC && publicRefused)
                return (
                    <Box
                        component="label"
                        key={m}
                        sx={{
                            "position": "relative",
                            "minHeight": "44px",
                            "display": "flex",
                            "alignItems": "center",
                            "justifyContent": "center",
                            "fontSize": "14px",
                            "fontWeight": 600,
                            "bgcolor": checked ? BOARD.navy : BOARD.white,
                            "color": checked ? BOARD.white : BOARD.muted,
                            "cursor": off ? "not-allowed" : "pointer",
                            "&:has(input:focus-visible)": focusRing,
                        }}
                    >
                        <input
                            type="radio"
                            name={name}
                            value={m}
                            checked={checked}
                            disabled={off}
                            onChange={() => onChange?.(m)}
                            style={{
                                position: "absolute",
                                opacity: 0,
                                width: 1,
                                height: 1,
                                margin: 0,
                            }}
                        />
                        {t(`transparencyTab.mode.${m}`)}
                    </Box>
                )
            })}
        </Box>
    )
}

export interface IVisibilityCardProps {
    mode: ETransparencyMode
    refusal?: string
    locked: boolean
    enabling: boolean
    onEnable: (mode: ETransparencyMode) => void
}

export const VisibilityCard: React.FC<IVisibilityCardProps> = ({
    mode,
    refusal,
    locked,
    enabling,
    onEnable,
}) => {
    const {t} = useTranslation()
    const publicRefused = Boolean(refusal)
    const [choice, setChoice] = useState(
        publicRefused ? ETransparencyMode.PRIVATE : ETransparencyMode.PUBLIC
    )
    const [confirming, setConfirming] = useState(false)
    const chosen = publicRefused && choice === ETransparencyMode.PUBLIC ? undefined : choice
    const disabled = mode === ETransparencyMode.DISABLED
    const choosing = disabled && !locked

    const aside = locked ? (
        <Box
            component="span"
            data-testid="transparency-locked"
            sx={{display: "inline-flex", alignItems: "center", gap: "6px"}}
        >
            <LockIcon />
            {t("transparencyTab.overview.visibility.locked")}
        </Box>
    ) : !disabled ? (
        t("transparencyTab.overview.visibility.fixed")
    ) : null

    return (
        <BoardSection
            title={t("transparencyTab.overview.visibility.title")}
            aside={aside}
            testId="overview-visibility"
        >
            {refusal && disabled ? <RefusalAlert code={refusal} /> : null}
            {choosing ? (
                <Box
                    data-testid="enable-transparency"
                    sx={{display: "flex", flexDirection: "column", gap: "12px"}}
                >
                    <VisibilityChoice
                        value={chosen}
                        disabled={enabling}
                        publicRefused={publicRefused}
                        onChange={setChoice}
                    />
                    <Typography sx={{fontSize: "13.5px", lineHeight: 1.5, color: BOARD.muted}}>
                        {chosen
                            ? t(`transparencyTab.mode.${chosen}Help`)
                            : t("transparencyTab.mode.choose")}
                    </Typography>
                    <Box>
                        <Button
                            disabled={!chosen || enabling}
                            onClick={() => setConfirming(true)}
                            data-testid="enable-transparency-button"
                            sx={filledButton}
                        >
                            {t("transparencyTab.mode.enable")}
                        </Button>
                    </Box>
                    <Dialog open={confirming} onClose={() => setConfirming(false)}>
                        <DialogTitle>
                            {t("transparencyTab.mode.confirmTitle", {
                                mode: chosen ? t(`transparencyTab.mode.${chosen}`) : "",
                            })}
                        </DialogTitle>
                        <DialogContent>
                            <DialogContentText>
                                {t("transparencyTab.mode.confirmText")}
                            </DialogContentText>
                        </DialogContent>
                        <DialogActions>
                            <Button onClick={() => setConfirming(false)} sx={outlinedButton}>
                                {t("transparencyTab.cancel")}
                            </Button>
                            <Button
                                data-testid="confirm-enable-transparency"
                                sx={filledButton}
                                onClick={() => {
                                    setConfirming(false)
                                    if (chosen) onEnable(chosen)
                                }}
                            >
                                {t("transparencyTab.mode.enable")}
                            </Button>
                        </DialogActions>
                    </Dialog>
                </Box>
            ) : (
                <>
                    <VisibilityChoice
                        value={disabled ? undefined : mode}
                        disabled
                        publicRefused={publicRefused}
                    />
                    <Typography sx={{fontSize: "13.5px", lineHeight: 1.5, color: BOARD.muted}}>
                        {disabled
                            ? t("transparencyTab.mode.locked")
                            : t(`transparencyTab.mode.${mode}Help`)}
                    </Typography>
                </>
            )}
        </BoardSection>
    )
}

// -- Redactions, monitors and evidence --------------------------------------------------

export const PolicyCards: React.FC<{facts: IPolicyFacts}> = ({facts}) => {
    const {t} = useTranslation()
    const key = (k: string) => `transparencyTab.overview.${k}`
    const monitors = facts.monitors.map((m) =>
        m.party ? t(key("named"), {name: m.name, party: m.party}) : m.name
    )
    return (
        <>
            <BoardSection title={t(key("redactions.title"))} testId="overview-redactions">
                <Typography sx={{fontSize: "13.5px", lineHeight: 1.5}}>
                    {facts.redaction
                        ? t(key("redactions.approvers"), {
                              k: facts.redaction.k,
                              count: facts.redaction.names.length,
                              names: facts.redaction.names.join(", "),
                          })
                        : t(key("redactions.none"))}
                </Typography>
            </BoardSection>
            <BoardSection title={t(key("monitors.title"))} testId="overview-monitors">
                <Typography sx={{fontSize: "13.5px", lineHeight: 1.5}}>
                    {monitors.length > 0
                        ? t(key("monitors.list"), {
                              count: monitors.length,
                              names: monitors.join(", "),
                              quorum: facts.monitorQuorum,
                          })
                        : t(key("monitors.none"))}
                </Typography>
            </BoardSection>
            <BoardSection title={t(key("evidence.title"))} testId="overview-evidence">
                <Typography sx={{fontSize: "13.5px", lineHeight: 1.5}}>
                    {t(key("evidence.help"))}
                </Typography>
            </BoardSection>
        </>
    )
}
