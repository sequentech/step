// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useMemo} from "react"
import {Box, Typography} from "@mui/material"
import {styled} from "@mui/material/styles"
import {useTranslation} from "react-i18next"
import {StyledButton, theme} from "@sequentech/ui-essentials"
import {useKeptReceipts} from "../../hooks/useCastReceipt"
import {SettingsContext} from "../../providers/SettingsContextProvider"
import {vitrineLanguage} from "../../services/vitrine/language"
import {readTransparency} from "../../services/vitrine/presentation"
import {ETransparencyMode} from "../../services/vitrine/types"
import {openVerifier} from "./CastReceipt"
import {VitrineReceiptView} from "./VitrineElements"

const KeptContainer = styled(Box)`
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin: 25px 0;
`

export interface IKeptReceiptsProps {
    eventId?: string
    /** The election event's `presentation`. */
    presentation?: unknown
    /** The election names, by ID. */
    electionNames?: Record<string, string>
}

/** The receipts of the voter's ballots kept on this device, shown after
 * login on the election list (step-integration.md, section 2.6). */
export const KeptReceipts: React.FC<IKeptReceiptsProps> = ({
    eventId,
    presentation,
    electionNames,
}) => {
    const {t, i18n} = useTranslation()
    const {globalSettings} = useContext(SettingsContext)
    const transparency = useMemo(() => readTransparency(presentation), [presentation])
    const {receipts, forgetAll} = useKeptReceipts(eventId, transparency)
    const lang = vitrineLanguage(i18n.language)
    if (transparency.mode === ETransparencyMode.DISABLED || receipts.length === 0) return null

    return (
        <KeptContainer className="vitrine-kept-receipts">
            <Typography
                className="vitrine-kept-receipts-title"
                variant="h5"
                component="h2"
                fontSize="18px"
                fontWeight="bold"
            >
                {t("vitrine.storedReceipts.title")}
            </Typography>
            <Typography
                className="vitrine-kept-receipts-description"
                variant="body2"
                component="div"
                sx={{color: theme.palette.customGrey.main}}
            >
                {t("vitrine.storedReceipts.description")}
            </Typography>
            {receipts.map(({kept, view}) => (
                <Box className="vitrine-kept-receipt" key={kept.electionId}>
                    {electionNames?.[kept.electionId] ? (
                        <Typography
                            className="vitrine-kept-receipt-election"
                            variant="body2"
                            component="h3"
                            fontWeight="bold"
                        >
                            {electionNames[kept.electionId]}
                        </Typography>
                    ) : null}
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
            ))}
            <Box className="vitrine-kept-receipts-actions">
                <StyledButton
                    className="vitrine-kept-receipts-forget-button"
                    variant="secondary"
                    onClick={() => void forgetAll()}
                >
                    {t("vitrine.storedReceipts.forget")}
                </StyledButton>
            </Box>
        </KeptContainer>
    )
}
