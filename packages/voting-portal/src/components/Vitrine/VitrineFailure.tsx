// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The page when the receipt can't be confirmed (Vitrine's design board Alert):
// what happened, the evidence this device holds, and what the voter can do.
// The evidence list is the verifier's own result, so it names only what the
// signed proofs show. The kit's `<vitrine-alert>` offers to send a proven
// failure's evidence file to the observers; when the portal builds that file,
// it replaces `FailureEvidence` and the receipt's disclosure in this page.

import React, {useMemo} from "react"
import {Box, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {StyledButton} from "@sequentech/ui-essentials"
import {wordsFor} from "@vitrine/ui"
import type {Language} from "@vitrine/ui"
import {EReceiptNotice} from "../../services/vitrine/castReceipt"
import type {IFailure} from "../../services/vitrine/confirmation"
import {ETransparencyMode} from "../../services/vitrine/types"
import {BOARD, EGlyph, Glyph, headingSx, quietTextSx} from "./board"

const rowSx = {
    display: "flex",
    gap: "12px",
    padding: "12px 16px",
    borderTop: `1px solid ${BOARD.border}`,
} as const

export const FailureEvidence: React.FC<{failure: IFailure; lang: Language}> = ({failure, lang}) => {
    const {t} = useTranslation()
    const words = useMemo(() => wordsFor(lang), [lang])
    const items = failure.problems.length + (failure.entry ? 1 : 0)

    return (
        <Box
            className="vitrine-failure-evidence"
            component="section"
            aria-label={t("vitrine.failure.evidenceTitle")}
            sx={{border: `1px solid ${BOARD.border}`}}
        >
            <Box
                className="vitrine-failure-evidence-head"
                sx={{
                    display: "flex",
                    justifyContent: "space-between",
                    alignItems: "center",
                    gap: "12px",
                    padding: "12px 16px",
                    bgcolor: BOARD.tint,
                }}
            >
                <Typography
                    className="vitrine-failure-evidence-title"
                    component="h2"
                    sx={{fontSize: "13px", fontWeight: 700, color: BOARD.text}}
                >
                    {t("vitrine.failure.evidenceTitle")}
                </Typography>
                <Typography
                    className="vitrine-failure-evidence-count"
                    component="span"
                    sx={{fontSize: "12px", color: BOARD.muted}}
                >
                    {t("vitrine.failure.evidenceCount", {count: items})}
                </Typography>
            </Box>
            <Box className="vitrine-failure-evidence-list" component="ul" sx={{m: 0, p: 0}}>
                {failure.entry ? (
                    <Box className="vitrine-failure-receipt" component="li" sx={{...rowSx}}>
                        <Box
                            className="vitrine-failure-row-icon"
                            component="span"
                            sx={{color: BOARD.navy, mt: "1px"}}
                        >
                            <Glyph glyph={EGlyph.DOCUMENT} size={18} />
                        </Box>
                        <Box className="vitrine-failure-row-text" sx={{minWidth: 0}}>
                            <Typography
                                className="vitrine-failure-row-title"
                                component="div"
                                sx={{fontSize: "13px", fontWeight: 600, color: BOARD.text}}
                            >
                                {failure.entry.index !== undefined
                                    ? t("vitrine.failure.receiptEntry", {
                                          index: failure.entry.index.toLocaleString(lang),
                                      })
                                    : t("vitrine.failure.receiptPromise")}
                            </Typography>
                            <Typography
                                className="vitrine-failure-row-sub"
                                component="div"
                                sx={{fontSize: "12px", color: BOARD.muted}}
                            >
                                {t("vitrine.failure.receiptSigned")}
                            </Typography>
                        </Box>
                    </Box>
                ) : null}
                {failure.problems.map((problem) => (
                    <Box
                        className="vitrine-failure-problem"
                        component="li"
                        key={problem.tier}
                        data-tier={problem.tier}
                        sx={{...rowSx}}
                    >
                        <Box
                            className="vitrine-failure-row-icon"
                            component="span"
                            sx={{color: BOARD.warning, mt: "1px"}}
                        >
                            <Glyph glyph={EGlyph.CONTRADICTION} size={18} />
                        </Box>
                        <Box className="vitrine-failure-row-text" sx={{minWidth: 0}}>
                            <Typography
                                className="vitrine-failure-row-title"
                                component="div"
                                sx={{fontSize: "13px", fontWeight: 600, color: BOARD.text}}
                            >
                                {words.t(`tier-${problem.tier}-failed`)}
                            </Typography>
                            {problem.reasons.map((reason, i) => (
                                <Typography
                                    className="vitrine-failure-row-reason"
                                    component="div"
                                    key={`${reason.code}-${i}`}
                                    sx={{fontSize: "12px", color: BOARD.warningText}}
                                >
                                    {words.reason(reason)}
                                    {reason.objects && reason.objects.length > 0
                                        ? ` ${words.t("steps-objects", {objects: reason.objects.join(", ")})}`
                                        : ""}
                                </Typography>
                            ))}
                        </Box>
                    </Box>
                ))}
            </Box>
        </Box>
    )
}

export interface IVitrineFailureProps {
    notice: EReceiptNotice
    mode: ETransparencyMode
    failure?: IFailure
    lang: Language
    /** The kit's receipt, for whoever wants every check. */
    receipt: React.ReactNode
    onCheckIndependently?: () => void
    onSave?: () => void
}

export const VitrineFailure: React.FC<IVitrineFailureProps> = ({
    notice,
    mode,
    failure,
    lang,
    receipt,
    onCheckIndependently,
    onSave,
}) => {
    const {t} = useTranslation()
    const record = mode === ETransparencyMode.PRIVATE ? "Private" : "Public"
    return (
        <Box
            className="vitrine-failure"
            data-notice={notice}
            sx={{display: "flex", flexDirection: "column", gap: "18px"}}
        >
            <Box
                className="vitrine-failure-head"
                role="alert"
                sx={{display: "flex", flexDirection: "column", gap: "14px"}}
            >
                <Box
                    className="vitrine-failure-icon"
                    aria-hidden="true"
                    sx={{
                        width: 48,
                        height: 48,
                        borderRadius: "50%",
                        border: `2.5px solid ${BOARD.warning}`,
                        bgcolor: BOARD.warningTint,
                        color: BOARD.warning,
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "center",
                    }}
                >
                    <Glyph glyph={EGlyph.EXCLAIM} size={28} stroke={3.2} />
                </Box>
                <Typography
                    className="vitrine-failure-title screen-title"
                    component="h1"
                    sx={{...headingSx, fontSize: "23px", lineHeight: 1.25, color: BOARD.text}}
                >
                    {t(`vitrine.failure.title${record}`)}
                </Typography>
                <Typography
                    className="vitrine-failure-explanation"
                    component="p"
                    sx={{margin: 0, fontSize: "14px", lineHeight: 1.5, color: BOARD.body}}
                >
                    {notice === EReceiptNotice.MISSING
                        ? t("vitrine.receipt.missing")
                        : t("vitrine.failure.contradiction")}
                </Typography>
            </Box>
            {failure ? <FailureEvidence failure={failure} lang={lang} /> : null}
            <Typography
                className="vitrine-failure-observers"
                component="p"
                sx={{...quietTextSx, margin: 0, fontSize: "13px", lineHeight: 1.5}}
            >
                {t("vitrine.failure.observers")}
            </Typography>
            <Box
                className="vitrine-failure-actions"
                sx={{display: "flex", flexDirection: "column", gap: "10px"}}
            >
                {onCheckIndependently ? (
                    <StyledButton
                        className="vitrine-failure-check-button"
                        onClick={onCheckIndependently}
                        sx={{minHeight: "48px", fontWeight: 600}}
                    >
                        {t("vitrine.failure.check")}
                    </StyledButton>
                ) : null}
                {onSave ? (
                    <StyledButton
                        className="vitrine-failure-save-button"
                        variant="secondary"
                        onClick={onSave}
                        sx={{minHeight: "48px", fontWeight: 600}}
                    >
                        {t("vitrine.failure.save")}
                    </StyledButton>
                ) : null}
            </Box>
            <Box
                className="vitrine-failure-next"
                component="details"
                sx={{borderTop: `1px solid ${BOARD.border}`, padding: "4px 0"}}
            >
                <Box
                    className="vitrine-failure-next-summary"
                    component="summary"
                    sx={{
                        minHeight: "44px",
                        display: "flex",
                        alignItems: "center",
                        cursor: "pointer",
                        fontSize: "14px",
                        fontWeight: 600,
                        color: BOARD.navy,
                        textDecoration: "underline",
                    }}
                >
                    {t("vitrine.failure.nextTitle")}
                </Box>
                <Typography
                    className="vitrine-failure-next-text"
                    component="p"
                    sx={{...quietTextSx, margin: "0 0 8px"}}
                >
                    {t("vitrine.failure.nextText")}
                </Typography>
            </Box>
            <Box
                className="vitrine-failure-receipt-details"
                component="details"
                sx={{borderTop: `1px solid ${BOARD.border}`, padding: "4px 0"}}
            >
                <Box
                    className="vitrine-failure-receipt-summary"
                    component="summary"
                    sx={{
                        minHeight: "44px",
                        display: "flex",
                        alignItems: "center",
                        cursor: "pointer",
                        fontSize: "14px",
                        fontWeight: 600,
                        color: BOARD.navy,
                        textDecoration: "underline",
                    }}
                >
                    {t("vitrine.failure.showReceipt")}
                </Box>
                <Box className="vitrine-failure-receipt-body" sx={{marginTop: "8px"}}>
                    {receipt}
                </Box>
            </Box>
        </Box>
    )
}
