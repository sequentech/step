// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The confirmation screen's page around Vitrine's receipt (step-integration.md,
// section 2.6; the design board Main): the voting steps, a headline that
// changes once the witnesses confirm the ballot, the kit's receipt checked by
// this website, a way to check it on another device, and the actions. The
// receipt card is the kit's element; its words and look are the kit's.

import React, {useContext, useEffect, useId, useMemo, useState} from "react"
import {Box, ButtonBase, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {QRCode, VisuallyHidden} from "@sequentech/ui-essentials"
import type {ChainResult, OverallStatus} from "@vitrine/ui"
import {useCastReceipt} from "../../hooks/useCastReceipt"
import {AuthContext} from "../../providers/AuthContextProvider"
import {SettingsContext} from "../../providers/SettingsContextProvider"
import {EReceiptNotice, IReceiptView} from "../../services/vitrine/castReceipt"
import {
    confirmationPhase,
    EConfirmationPhase,
    failureOf,
    receiptFileName,
} from "../../services/vitrine/confirmation"
import {RECEIPT_MEDIA_TYPE, saveTextFile} from "../../services/vitrine/file"
import {openVerifier, receiptReferenceUrl} from "../../services/vitrine/handoff"
import {vitrineLanguage} from "../../services/vitrine/language"
import {chainReceipt} from "../../services/vitrine/receipt"
import {loadVitrineSdk} from "../../services/vitrine/sdk"
import {EReceiptKeeping} from "../../services/vitrine/storage"
import {ETransparencyMode} from "../../services/vitrine/types"
import type {IConfirmationVitrine} from "../../store/castVotes/confirmationScreenDataSlice"
import {BOARD, EGlyph, focusSx, Glyph, headingSx, quietTextSx, VitrineBrand} from "./board"
import {VitrineFailure} from "./VitrineFailure"
import {VitrineReceiptView, VitrineStatusView} from "./VitrineElements"
import {ELECTION_PROFILE_WORDS} from "../../services/vitrine/profileWords"

const NOTICE_STATE: Record<EReceiptNotice, OverallStatus> = {
    [EReceiptNotice.NONE]: "verified",
    [EReceiptNotice.WAITING]: "pending",
    [EReceiptNotice.MISSING]: "failed",
    [EReceiptNotice.UNCHECKED]: "invalid",
}

/** The receipt's reference for its QR code, built on this device: showing
 * and printing it makes no request. */
const useReceiptReference = (verifierUrl: string, view?: IReceiptView): string | undefined => {
    const [reference, setReference] = useState<string>()
    useEffect(() => {
        const {chain, own, policy} = view ?? {}
        setReference(undefined)
        if (!chain || !own || !policy || !verifierUrl) return
        let active = true
        const build = async () => {
            try {
                const sdk = await loadVitrineSdk()
                const receipt = chainReceipt(sdk, chain)
                const url =
                    receipt &&
                    receiptReferenceUrl(sdk.pure, verifierUrl, policy, receipt, own.entry)
                if (active) setReference(url)
            } catch {
                if (active) setReference(undefined)
            }
        }
        void build()
        return () => {
            active = false
        }
    }, [verifierUrl, view])
    return reference
}

/** "Check it on another device": the receipt's reference as a QR code. */
const CheckElsewhere: React.FC<{reference?: string}> = ({reference}) => {
    const {t} = useTranslation()
    const [open, setOpen] = useState(false)
    const panel = useId()
    if (!reference) return null
    return (
        <Box className="vitrine-receipt-reference">
            <ButtonBase
                className="vitrine-elsewhere-toggle"
                aria-expanded={open}
                aria-controls={panel}
                onClick={() => setOpen(!open)}
                sx={{
                    ...focusSx,
                    "width": "100%",
                    "minHeight": "44px",
                    "display": "flex",
                    "alignItems": "center",
                    "justifyContent": "space-between",
                    "gap": "12px",
                    "color": BOARD.navy,
                    "fontSize": "14px",
                    "fontWeight": 600,
                    "textAlign": "left",
                    "borderBottom": `1px solid ${BOARD.border}`,
                    "& .vitrine-elsewhere-chevron": {
                        transition: "transform 0.15s",
                        transform: open ? "rotate(90deg)" : "none",
                    },
                    "@media (prefers-reduced-motion: reduce)": {
                        "& .vitrine-elsewhere-chevron": {transition: "none"},
                    },
                }}
            >
                <Box
                    className="vitrine-elsewhere-label"
                    component="span"
                    sx={{display: "flex", alignItems: "center", gap: "10px"}}
                >
                    <Glyph glyph={EGlyph.QR} />
                    {t("vitrine.confirmation.elsewhere")}
                </Box>
                <Glyph className="vitrine-elsewhere-chevron" glyph={EGlyph.CHEVRON} size={16} />
            </ButtonBase>
            <Box
                className="vitrine-elsewhere-panel"
                id={panel}
                hidden={!open}
                sx={{
                    display: open ? "flex" : "none",
                    flexDirection: "column",
                    alignItems: "flex-start",
                    gap: "8px",
                    padding: "12px 0 4px",
                }}
            >
                <Typography
                    className="vitrine-receipt-reference-text"
                    component="p"
                    id="vitrine-receipt-reference-description"
                    sx={{...quietTextSx, margin: 0}}
                >
                    {t("vitrine.receipt.reference")}
                </Typography>
                <QRCode ariaLabelledby="vitrine-receipt-reference-description" value={reference} />
            </Box>
        </Box>
    )
}

/** What the page offers beside the receipt. */
export interface IReceiptActions {
    /** Saving the receipt as a file: shown, and enabled once there is one. */
    save?: {onClick?: () => void}
}

export interface ICastReceiptProps {
    eventId: string
    electionId: string
    /** The portal's own ballot ID. */
    ballotId: string
    vitrine: IConfirmationVitrine
    /** The voting steps. */
    progress: React.ReactNode
    /** The help button beside the headline. */
    help?: React.ReactNode
    /** The portal's own ballot ID and ballot locator, below the actions. */
    details?: React.ReactNode
    renderActions: (receipt: IReceiptActions) => React.ReactNode
}

/** The confirmation screen's body when the event keeps a Vitrine record:
 * checked by this website, with "Check independently" as the receipt's primary
 * action. */
export const CastReceipt: React.FC<ICastReceiptProps> = ({
    eventId,
    electionId,
    ballotId,
    vitrine,
    progress,
    help,
    details,
    renderActions,
}) => {
    const {t, i18n} = useTranslation()
    const {isKiosk} = useContext(AuthContext)
    const {globalSettings} = useContext(SettingsContext)
    const target = useMemo(() => ({eventId, electionId, ballotId}), [eventId, electionId, ballotId])
    const {view, keeping, kept, setKeeping} = useCastReceipt(target, vitrine)
    const reference = useReceiptReference(globalSettings.BALLOT_VERIFIER_URL, view)
    const [computed, setComputed] = useState<ChainResult>()
    const lang = vitrineLanguage(i18n.language)

    const notice = view?.notice ?? EReceiptNotice.NONE
    const result = view?.result ?? computed
    const phase = confirmationPhase(notice, result)
    const mode =
        vitrine.transparency.mode === ETransparencyMode.PRIVATE
            ? ETransparencyMode.PRIVATE
            : ETransparencyMode.PUBLIC
    const message = notice === EReceiptNotice.NONE ? undefined : t(`vitrine.receipt.${notice}`)
    const keepingNow = keeping === EReceiptKeeping.KEEP
    const chain = view?.chain
    const onSave = chain
        ? () => saveTextFile(receiptFileName(ballotId), chain, RECEIPT_MEDIA_TYPE)
        : undefined
    const checkIndependently = (detailChain?: string | null) => {
        const checked = detailChain ?? chain
        if (view && checked) void openVerifier(globalSettings.BALLOT_VERIFIER_URL, view, checked)
    }
    const headline = t(
        phase === EConfirmationPhase.RECORDED
            ? `vitrine.confirmation.recorded${mode === ETransparencyMode.PRIVATE ? "Private" : "Public"}`
            : "vitrine.confirmation.cast"
    )

    const receipt = (
        <VitrineReceiptView
            className="vitrine-receipt"
            where="page"
            bare
            profileWords={ELECTION_PROFILE_WORDS}
            chain={view?.chain}
            policy={view?.policy}
            expect={view?.expect}
            result={view?.result}
            lang={lang}
            onResult={setComputed}
            onCheckIndependently={(detail) => checkIndependently(detail.chain)}
        />
    )

    return (
        <VitrineBrand className="vitrine-confirmation" data-phase={phase}>
            {phase === EConfirmationPhase.FAILED ? null : progress}
            {phase === EConfirmationPhase.FAILED ? (
                <VitrineFailure
                    notice={notice}
                    mode={mode}
                    failure={failureOf(result)}
                    lang={lang}
                    receipt={receipt}
                    onCheckIndependently={chain ? () => checkIndependently() : undefined}
                    onSave={onSave}
                />
            ) : (
                <>
                    <Box
                        className="vitrine-confirmation-head"
                        sx={{display: "flex", flexDirection: "column", gap: "6px"}}
                    >
                        <Box
                            className="vitrine-confirmation-title-row"
                            sx={{display: "flex", alignItems: "flex-start", gap: "12px"}}
                        >
                            <Typography
                                className="vitrine-confirmation-title screen-title"
                                component="h1"
                                sx={headingSx}
                            >
                                {headline}
                            </Typography>
                            {help}
                        </Box>
                        <Typography
                            className="vitrine-receipt-description"
                            component="p"
                            sx={{...quietTextSx, margin: 0}}
                        >
                            {t("vitrine.confirmation.keep")}
                        </Typography>
                    </Box>
                    <VisuallyHidden className="vitrine-confirmation-status" role="status">
                        {headline}
                    </VisuallyHidden>
                    {message ? (
                        <VitrineStatusView
                            className="vitrine-receipt-notice"
                            state={NOTICE_STATE[notice]}
                            message={message}
                            lang={lang}
                            fallback={
                                <Typography
                                    className="vitrine-receipt-notice-text"
                                    component="p"
                                    sx={{...quietTextSx, margin: 0}}
                                >
                                    {message}
                                </Typography>
                            }
                        />
                    ) : null}
                    <Box className="vitrine-receipt-container">{receipt}</Box>
                    <CheckElsewhere reference={reference} />
                </>
            )}
            <Box className="vitrine-confirmation-actions" sx={{marginTop: "10px"}}>
                {renderActions(
                    phase === EConfirmationPhase.FAILED ? {} : {save: {onClick: onSave}}
                )}
            </Box>
            {isKiosk() ? null : (
                <Box
                    className="vitrine-receipt-storage"
                    sx={{
                        display: "flex",
                        flexWrap: "wrap",
                        alignItems: "center",
                        justifyContent: "space-between",
                        gap: "0 12px",
                    }}
                >
                    <Typography
                        className="vitrine-receipt-storage-text"
                        component="p"
                        sx={{...quietTextSx, margin: 0, fontSize: "12.5px"}}
                    >
                        {t(kept ? "vitrine.receipt.stored" : "vitrine.receipt.notStored")}
                    </Typography>
                    <ButtonBase
                        className="vitrine-receipt-keeping-button"
                        onClick={() =>
                            setKeeping(keepingNow ? EReceiptKeeping.FORGET : EReceiptKeeping.KEEP)
                        }
                        sx={{
                            ...focusSx,
                            minHeight: "44px",
                            fontSize: "13px",
                            fontWeight: 600,
                            color: BOARD.navy,
                            textDecoration: "underline",
                        }}
                    >
                        {t(keepingNow ? "vitrine.receipt.forget" : "vitrine.receipt.keep")}
                    </ButtonBase>
                </Box>
            )}
            {details}
        </VitrineBrand>
    )
}
