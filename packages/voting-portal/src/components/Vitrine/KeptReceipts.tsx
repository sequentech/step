// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The receipts of the voter's ballots kept on this device, after login (step-
// integration.md, section 2.6; the design board Revisit): whether each ballot
// is still in the record, and how the portal checks. The kit's receipt is one
// click away. What a tier shows is only what the verifier's result says.

import React, {useContext, useId, useMemo, useState} from "react"
import {Box, ButtonBase, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {wordsFor} from "@vitrine/ui"
import type {Language} from "@vitrine/ui"
import type {TierStatus} from "@vitrine/verify"
import {IKeptReceiptView, useKeptReceipts} from "../../hooks/useCastReceipt"
import {SettingsContext} from "../../providers/SettingsContextProvider"
import {ERevisitPhase, IRevisitSummary, revisitSummary} from "../../services/vitrine/confirmation"
import {vitrineLanguage} from "../../services/vitrine/language"
import {readTransparency} from "../../services/vitrine/presentation"
import {ETransparencyMode} from "../../services/vitrine/types"
import {BOARD, EGlyph, focusSx, Glyph, quietTextSx} from "./board"
import {openVerifier} from "./CastReceipt"
import {VitrineReceiptView} from "./VitrineElements"

const SECTION_WIDTH = "720px"

interface IPhaseStyle {
    border: string
    background: string
    /** The rules inside the card. */
    line: string
    title: string
    text: string
}

const PHASE_STYLE: Record<ERevisitPhase, IPhaseStyle> = {
    [ERevisitPhase.HOLDS]: {
        border: BOARD.greenLine,
        background: BOARD.greenTint,
        line: BOARD.greenLine,
        title: BOARD.greenInk,
        text: BOARD.greenBody,
    },
    [ERevisitPhase.WAITING]: {
        border: BOARD.border,
        background: BOARD.tint,
        line: BOARD.border,
        title: BOARD.navy,
        text: BOARD.body,
    },
    [ERevisitPhase.UNCHECKED]: {
        border: BOARD.border,
        background: BOARD.tint,
        line: BOARD.border,
        title: BOARD.navy,
        text: BOARD.body,
    },
    [ERevisitPhase.PROBLEM]: {
        border: BOARD.warning,
        background: BOARD.warningTint,
        line: "#FDBA74",
        title: BOARD.warningText,
        text: BOARD.body,
    },
}

const PhaseSeal: React.FC<{phase: ERevisitPhase}> = ({phase}) => {
    if (phase === ERevisitPhase.HOLDS) {
        return (
            <Box
                className="vitrine-kept-seal"
                aria-hidden="true"
                sx={{
                    flex: "none",
                    width: 40,
                    height: 40,
                    borderRadius: "50%",
                    border: `2.5px solid ${BOARD.green}`,
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                }}
            >
                <Box
                    className="vitrine-kept-seal-inner"
                    sx={{
                        width: 27,
                        height: 27,
                        borderRadius: "50%",
                        border: `2.5px solid ${BOARD.greenInk}`,
                        color: BOARD.greenInk,
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "center",
                    }}
                >
                    <Glyph glyph={EGlyph.CHECK} size={14} stroke={2.8} />
                </Box>
            </Box>
        )
    }
    const problem = phase === ERevisitPhase.PROBLEM
    const color = problem ? BOARD.warning : BOARD.subtle
    return (
        <Box
            className="vitrine-kept-seal"
            aria-hidden="true"
            sx={{
                flex: "none",
                width: 40,
                height: 40,
                borderRadius: "50%",
                border: `2.5px solid ${color}`,
                bgcolor: problem ? BOARD.warningTint : BOARD.white,
                color,
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
            }}
        >
            <Glyph glyph={problem ? EGlyph.EXCLAIM : EGlyph.REFRESH} size={22} stroke={2.6} />
        </Box>
    )
}

interface ITierRow {
    tier: string
    label: string
    value: string
    done: boolean
}

const TierList: React.FC<{
    summary: IRevisitSummary
    mode: ETransparencyMode
    lang: Language
    style: IPhaseStyle
}> = ({summary, mode, lang, style}) => {
    const {t} = useTranslation()
    const words = useMemo(() => wordsFor(lang), [lang])
    const status = (s: TierStatus) => words.t(`tier-status-${s}`)
    const rows: ITierRow[] = []
    if (summary.witnesses) {
        rows.push({
            tier: "witnesses",
            label: t("vitrine.storedReceipts.witnesses"),
            value: status(summary.witnesses),
            done: summary.witnesses === "verified",
        })
    }
    if (summary.hubs) {
        const {agree, total} = summary.hubs
        rows.push({
            tier: "hubs",
            label: t(
                mode === ETransparencyMode.PRIVATE
                    ? "vitrine.storedReceipts.hubsPrivate"
                    : "vitrine.storedReceipts.hubs"
            ),
            value:
                total > 0
                    ? t("vitrine.storedReceipts.hubsAgree", {agree, total})
                    : status(summary.hubs.status),
            done: summary.hubs.status === "verified",
        })
    }
    if (summary.browser) {
        rows.push({
            tier: "browser",
            label: t("vitrine.storedReceipts.browser"),
            value:
                summary.browser === "verified"
                    ? t("vitrine.storedReceipts.anchored")
                    : status(summary.browser),
            done: summary.browser === "verified",
        })
    }
    if (rows.length === 0) return null
    return (
        <Box
            className="vitrine-kept-tiers"
            component="dl"
            sx={{
                m: 0,
                padding: "10px 0 0",
                borderTop: `1px solid ${style.line}`,
                display: "flex",
                flexDirection: "column",
                gap: "8px",
            }}
        >
            {rows.map((row) => (
                <Box
                    className="vitrine-kept-tier"
                    key={row.tier}
                    data-tier={row.tier}
                    sx={{
                        display: "flex",
                        alignItems: "center",
                        gap: "10px",
                        fontSize: "13px",
                        lineHeight: 1.35,
                        color: style.text,
                    }}
                >
                    <Box
                        className="vitrine-kept-tier-label"
                        component="dt"
                        sx={{
                            flex: 1,
                            display: "flex",
                            alignItems: "center",
                            gap: "10px",
                            fontWeight: 600,
                            color: style.title,
                        }}
                    >
                        <Box
                            className="vitrine-kept-tier-mark"
                            component="span"
                            aria-hidden="true"
                            sx={{
                                flex: "none",
                                width: 16,
                                height: 16,
                                boxSizing: "border-box",
                                borderRadius: "50%",
                                bgcolor: row.done ? BOARD.green : "transparent",
                                border: row.done ? "none" : `2px dashed ${BOARD.subtle}`,
                                color: BOARD.white,
                                display: "flex",
                                alignItems: "center",
                                justifyContent: "center",
                            }}
                        >
                            {row.done ? (
                                <Glyph glyph={EGlyph.CHECK} size={10} stroke={3.2} />
                            ) : null}
                        </Box>
                        {row.label}
                    </Box>
                    <Box className="vitrine-kept-tier-value" component="dd" sx={{m: 0}}>
                        {row.value}
                    </Box>
                </Box>
            ))}
        </Box>
    )
}

const KeptReceiptCard: React.FC<{
    entry: IKeptReceiptView
    name?: string
    mode: ETransparencyMode
    lang: Language
}> = ({entry, name, mode, lang}) => {
    const {t} = useTranslation()
    const {globalSettings} = useContext(SettingsContext)
    const [open, setOpen] = useState(false)
    const panel = useId()
    const {view} = entry
    const summary = revisitSummary(view?.result)
    const phase = summary.phase
    const style = PHASE_STYLE[phase]
    const record = mode === ETransparencyMode.PRIVATE ? "Private" : "Public"
    const title = t(
        phase === ERevisitPhase.HOLDS
            ? `vitrine.storedReceipts.holds${record}`
            : phase === ERevisitPhase.PROBLEM
              ? `vitrine.failure.title${record}`
              : `vitrine.storedReceipts.${phase}`
    )
    const help = t(`vitrine.storedReceipts.${phase}Help`)
    const words = view?.result?.words ?? []

    return (
        <Box
            className="vitrine-kept-receipt"
            component="section"
            data-phase={phase}
            aria-label={name ?? t("vitrine.storedReceipts.title")}
            sx={{
                border: `1px solid ${style.border}`,
                bgcolor: style.background,
                padding: "16px",
                display: "flex",
                flexDirection: "column",
                gap: "12px",
            }}
        >
            {name ? (
                <Typography
                    className="vitrine-kept-receipt-election"
                    component="div"
                    sx={{margin: 0, fontSize: "12px", fontWeight: 600, color: BOARD.muted}}
                >
                    {name}
                </Typography>
            ) : null}
            <Box className="vitrine-kept-head" sx={{display: "flex", gap: "14px"}}>
                <PhaseSeal phase={phase} />
                <Box
                    className="vitrine-kept-head-text"
                    sx={{display: "flex", flexDirection: "column", gap: "4px"}}
                >
                    <Typography
                        className="vitrine-kept-title"
                        component="h2"
                        sx={{
                            m: 0,
                            fontSize: "16px",
                            lineHeight: 1.3,
                            fontWeight: 700,
                            color: style.title,
                        }}
                    >
                        {title}
                    </Typography>
                    <Typography
                        className="vitrine-kept-help"
                        component="p"
                        sx={{margin: 0, fontSize: "13px", lineHeight: 1.45, color: style.text}}
                    >
                        {help}
                    </Typography>
                </Box>
            </Box>
            <TierList summary={summary} mode={mode} lang={lang} style={style} />
            <Box
                className="vitrine-kept-foot"
                sx={{
                    display: "flex",
                    flexWrap: "wrap",
                    alignItems: "center",
                    justifyContent: "space-between",
                    gap: "0 12px",
                    paddingTop: "10px",
                    borderTop: `1px solid ${style.line}`,
                }}
            >
                <Typography
                    className="vitrine-kept-words"
                    component="span"
                    lang={view?.result?.wordsLang}
                    sx={{margin: 0, fontSize: "13px", fontWeight: 600, color: style.title}}
                >
                    {words.join(" · ")}
                </Typography>
                <ButtonBase
                    className="vitrine-kept-view-button"
                    aria-expanded={open}
                    aria-controls={panel}
                    onClick={() => setOpen(!open)}
                    sx={{
                        ...focusSx,
                        minHeight: "44px",
                        fontSize: "13px",
                        fontWeight: 600,
                        color: style.title,
                        textDecoration: "underline",
                    }}
                >
                    {t(
                        open
                            ? "vitrine.storedReceipts.hideReceipt"
                            : "vitrine.storedReceipts.viewReceipt"
                    )}
                </ButtonBase>
            </Box>
            <Box
                className="vitrine-kept-receipt-panel"
                id={panel}
                hidden={!open}
                sx={{display: open ? "block" : "none"}}
            >
                <VitrineReceiptView
                    className="vitrine-kept-receipt-element"
                    where="page"
                    compact
                    chain={view?.chain}
                    policy={view?.policy}
                    expect={view?.expect}
                    result={view?.result}
                    lang={lang}
                    onCheckIndependently={(detail) => {
                        const chain = detail.chain ?? view?.chain
                        if (view && chain) {
                            void openVerifier(globalSettings.BALLOT_VERIFIER_URL, view, chain)
                        }
                    }}
                />
            </Box>
        </Box>
    )
}

const HowWeCheck: React.FC<{mode: ETransparencyMode}> = ({mode}) => {
    const {t} = useTranslation()
    const items = [
        {glyph: EGlyph.DOCUMENT, key: "howReceipt"},
        {glyph: EGlyph.REFRESH, key: "howCheck"},
        {
            glyph: EGlyph.NETWORK,
            key: mode === ETransparencyMode.PRIVATE ? "howWitnessesPrivate" : "howWitnesses",
        },
    ]
    return (
        <Box
            className="vitrine-how"
            component="section"
            aria-labelledby="vitrine-how-title"
            sx={{
                maxWidth: SECTION_WIDTH,
                borderTop: `1px solid ${BOARD.border}`,
                paddingTop: "18px",
                display: "flex",
                flexDirection: "column",
                gap: "14px",
            }}
        >
            <Typography
                className="vitrine-how-title"
                component="h2"
                id="vitrine-how-title"
                sx={{
                    m: 0,
                    fontSize: "12px",
                    fontWeight: 700,
                    letterSpacing: ".08em",
                    textTransform: "uppercase",
                    color: BOARD.muted,
                }}
            >
                {t("vitrine.storedReceipts.howTitle")}
            </Typography>
            <Box
                className="vitrine-how-list"
                component="ul"
                sx={{
                    m: 0,
                    p: 0,
                    listStyle: "none",
                    display: "flex",
                    flexDirection: "column",
                    gap: "16px",
                }}
            >
                {items.map((item) => (
                    <Box
                        className="vitrine-how-item"
                        component="li"
                        key={item.key}
                        sx={{display: "flex", gap: "14px", alignItems: "flex-start"}}
                    >
                        <Box
                            className="vitrine-how-icon"
                            component="span"
                            sx={{color: BOARD.navy, mt: "1px"}}
                        >
                            <Glyph glyph={item.glyph} size={22} />
                        </Box>
                        <Typography
                            className="vitrine-how-text"
                            component="span"
                            sx={{margin: 0, ...quietTextSx, color: BOARD.body}}
                        >
                            {t(`vitrine.storedReceipts.${item.key}`)}
                        </Typography>
                    </Box>
                ))}
            </Box>
        </Box>
    )
}

export interface IKeptReceiptsProps {
    eventId?: string
    /** The election event's `presentation`. */
    presentation?: unknown
    /** The election names, by ID. */
    electionNames?: Record<string, string>
    /** The page's own list of elections, shown between the receipts and how
     * they are checked. */
    children?: React.ReactNode
}

/** The receipts of the voter's ballots kept on this device, around the
 * page's list of elections. Without receipts, only the list. */
export const KeptReceipts: React.FC<IKeptReceiptsProps> = ({
    eventId,
    presentation,
    electionNames,
    children,
}) => {
    const {t, i18n} = useTranslation()
    const transparency = useMemo(() => readTransparency(presentation), [presentation])
    const {receipts, forgetAll} = useKeptReceipts(eventId, transparency)
    const lang = vitrineLanguage(i18n.language)
    if (transparency.mode === ETransparencyMode.DISABLED || receipts.length === 0) {
        return <>{children}</>
    }
    const mode =
        transparency.mode === ETransparencyMode.PRIVATE
            ? ETransparencyMode.PRIVATE
            : ETransparencyMode.PUBLIC

    return (
        <>
            <Box
                className="vitrine-kept-receipts"
                sx={{
                    "--vitrine-accent": BOARD.navy,
                    "--vitrine-text": BOARD.text,
                    "--vitrine-surface": BOARD.white,
                    "--vitrine-radius": "0px",
                    "maxWidth": SECTION_WIDTH,
                    "margin": "16px 0 24px",
                    "display": "flex",
                    "flexDirection": "column",
                    "gap": "14px",
                }}
            >
                {receipts.map((entry) => (
                    <KeptReceiptCard
                        key={entry.kept.electionId}
                        entry={entry}
                        name={electionNames?.[entry.kept.electionId]}
                        mode={mode}
                        lang={lang}
                    />
                ))}
                <Box className="vitrine-kept-receipts-actions">
                    <ButtonBase
                        className="vitrine-kept-receipts-forget-button"
                        onClick={() => void forgetAll()}
                        sx={{
                            ...focusSx,
                            minHeight: "44px",
                            fontSize: "13px",
                            fontWeight: 600,
                            color: BOARD.navy,
                            textDecoration: "underline",
                        }}
                    >
                        {t("vitrine.storedReceipts.forget")}
                    </ButtonBase>
                </Box>
            </Box>
            {children}
            <HowWeCheck mode={mode} />
        </>
    )
}
