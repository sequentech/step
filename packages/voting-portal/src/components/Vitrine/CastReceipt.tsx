// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useMemo, useState} from "react"
import {Box, Typography} from "@mui/material"
import {styled} from "@mui/material/styles"
import {useTranslation} from "react-i18next"
import {QRCode, StyledButton, theme} from "@sequentech/ui-essentials"
import type {OverallStatus} from "@vitrine/ui"
import {useCastReceipt} from "../../hooks/useCastReceipt"
import {AuthContext} from "../../providers/AuthContextProvider"
import {SettingsContext} from "../../providers/SettingsContextProvider"
import {EReceiptNotice, IReceiptView} from "../../services/vitrine/castReceipt"
import {checkIndependentlyUrl, receiptReferenceUrl} from "../../services/vitrine/handoff"
import {vitrineLanguage} from "../../services/vitrine/language"
import {chainReceipt} from "../../services/vitrine/receipt"
import {loadVitrineSdk} from "../../services/vitrine/sdk"
import {EReceiptKeeping} from "../../services/vitrine/storage"
import {ETransparencyMode} from "../../services/vitrine/types"
import type {IConfirmationVitrine} from "../../store/castVotes/confirmationScreenDataSlice"
import {VitrineReceiptView, VitrineStatusView} from "./VitrineElements"

const ReceiptContainer = styled(Box)`
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin: 0 0 25px;
`

const StorageLine = styled(Box)`
    display: flex;
    flex-direction: row;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
`

const ReferenceContainer = styled(Box)`
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
`

const NOTICE_STATE: Record<EReceiptNotice, OverallStatus> = {
    [EReceiptNotice.NONE]: "verified",
    [EReceiptNotice.WAITING]: "pending",
    [EReceiptNotice.MISSING]: "failed",
    [EReceiptNotice.UNCHECKED]: "invalid",
}

/** Opens the receipt in the deployment's ballot verifier, in a new tab that
 * knows nothing of this one. */
export const openVerifier = async (verifierUrl: string, view: IReceiptView, chain: string) => {
    const {own, policy, policyText} = view
    if (!own || !policy) return
    try {
        const {pure} = await loadVitrineSdk()
        const mode =
            policy.visibility === ETransparencyMode.PUBLIC
                ? ETransparencyMode.PUBLIC
                : ETransparencyMode.PRIVATE
        const url = checkIndependentlyUrl(pure, {
            verifierUrl,
            chain,
            entry: own.entry,
            mode,
            policy,
            ...(policyText ? {policyText} : {}),
        })
        window.open(url, "_blank", "noopener,noreferrer")
    } catch (error) {
        console.error("Vitrine: couldn't open the verifier:", error)
    }
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

export interface ICastReceiptProps {
    eventId: string
    electionId: string
    /** The portal's own ballot ID. */
    ballotId: string
    vitrine: IConfirmationVitrine
}

/** The receipt on the confirmation screen (step-integration.md, section
 * 2.6), checked by this website, with "Check independently" as the primary
 * action. */
export const CastReceipt: React.FC<ICastReceiptProps> = ({
    eventId,
    electionId,
    ballotId,
    vitrine,
}) => {
    const {t, i18n} = useTranslation()
    const {isKiosk} = useContext(AuthContext)
    const {globalSettings} = useContext(SettingsContext)
    const target = useMemo(() => ({eventId, electionId, ballotId}), [eventId, electionId, ballotId])
    const {view, keeping, kept, setKeeping} = useCastReceipt(target, vitrine)
    const reference = useReceiptReference(globalSettings.BALLOT_VERIFIER_URL, view)
    const lang = vitrineLanguage(i18n.language)
    if (vitrine.transparency.mode === ETransparencyMode.DISABLED) return null

    const notice = view?.notice ?? EReceiptNotice.NONE
    const message = notice === EReceiptNotice.NONE ? undefined : t(`vitrine.receipt.${notice}`)
    const keepingNow = keeping === EReceiptKeeping.KEEP

    return (
        <ReceiptContainer className="vitrine-receipt-container">
            <Typography
                className="vitrine-receipt-title"
                variant="h5"
                component="h2"
                fontSize="18px"
                fontWeight="bold"
            >
                {t("vitrine.receipt.title")}
            </Typography>
            <Typography
                className="vitrine-receipt-description"
                variant="body2"
                component="div"
                sx={{color: theme.palette.customGrey.main}}
            >
                {t("vitrine.receipt.description")}
            </Typography>
            {message ? (
                <VitrineStatusView
                    className="vitrine-receipt-notice"
                    state={NOTICE_STATE[notice]}
                    message={message}
                    lang={lang}
                    fallback={
                        <Typography
                            className="vitrine-receipt-notice-text"
                            variant="body2"
                            component="p"
                        >
                            {message}
                        </Typography>
                    }
                />
            ) : null}
            <VitrineReceiptView
                className="vitrine-receipt"
                where="page"
                chain={view?.chain}
                policy={view?.policy}
                expect={view?.expect}
                result={view?.result}
                lang={lang}
                onCheckIndependently={(detail) => {
                    const chain = detail.chain ?? view?.chain
                    if (view && chain)
                        void openVerifier(globalSettings.BALLOT_VERIFIER_URL, view, chain)
                }}
            />
            {reference ? (
                <ReferenceContainer className="vitrine-receipt-reference">
                    <Typography
                        className="vitrine-receipt-reference-text"
                        variant="body2"
                        component="p"
                        id="vitrine-receipt-reference-description"
                        sx={{color: theme.palette.customGrey.main}}
                    >
                        {t("vitrine.receipt.reference")}
                    </Typography>
                    <QRCode
                        ariaLabelledby="vitrine-receipt-reference-description"
                        value={reference}
                    />
                </ReferenceContainer>
            ) : null}
            {isKiosk() ? null : (
                <StorageLine className="vitrine-receipt-storage">
                    <Typography
                        className="vitrine-receipt-storage-text"
                        variant="body2"
                        component="p"
                        sx={{color: theme.palette.customGrey.main}}
                    >
                        {t(kept ? "vitrine.receipt.stored" : "vitrine.receipt.notStored")}
                    </Typography>
                    <StyledButton
                        className="vitrine-receipt-keeping-button"
                        variant="secondary"
                        onClick={() =>
                            setKeeping(keepingNow ? EReceiptKeeping.FORGET : EReceiptKeeping.KEEP)
                        }
                    >
                        {t(keepingNow ? "vitrine.receipt.forget" : "vitrine.receipt.keep")}
                    </StyledButton>
                </StorageLine>
            )}
        </ReceiptContainer>
    )
}
