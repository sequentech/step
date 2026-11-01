// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// What the user compares with the official election notice before pinning a
// trust policy: its policy ID in the groups of a Tracking ID (the first four
// are what people compare), its event and its witnesses, in the UI kit's own
// words. Everything here comes from the policy and is shown as text.

import React from "react"
import Box from "@mui/material/Box"
import Typography from "@mui/material/Typography"
import {styled} from "@mui/material/styles"
import {useTranslation} from "react-i18next"
import type {PolicySummary} from "@vitrine/verify"
import type {Language} from "@vitrine/ui"
import type {TVitrineUi} from "../../services/vitrine/sdk"

const Facts = styled("dl")`
    margin: 8px 0;
    display: grid;
    grid-template-columns: minmax(8em, max-content) 1fr;
    gap: 4px 16px;

    dt {
        font-weight: bold;
    }
    dd {
        margin: 0;
        overflow-wrap: anywhere;
    }
    ul {
        margin: 0;
        padding-left: 1.2em;
    }
    code {
        font-size: 1.05em;
        letter-spacing: 0.02em;
    }
    code strong {
        font-size: 1.15em;
    }
`

export interface IPolicyFactsProps {
    ui: TVitrineUi
    lang: Language
    summary: PolicySummary
    /** The policy ID in 13 groups of four joined by hyphens. */
    idDisplay: string
    /** Its first four groups, which people compare with the notice. */
    shortId: string
    /** Show the heading and hint for the comparison with the notice. */
    compare?: boolean
    /** Show the witnesses. */
    witnesses?: boolean
}

export const PolicyFacts: React.FC<IPolicyFactsProps> = ({
    ui,
    lang,
    summary,
    idDisplay,
    shortId,
    compare = false,
    witnesses = false,
}) => {
    const words = ui.wordsFor(lang)
    const {t} = useTranslation()
    return (
        <Box className="vitrine-policy-facts" data-testid="policy-facts">
            {compare ? (
                <>
                    <Typography variant="h6" component="h3">
                        {words.t("receipt-compare-heading")}
                    </Typography>
                    <Typography variant="body2">{words.t("receipt-compare-hint")}</Typography>
                </>
            ) : null}
            <Facts>
                <dt>{words.t("receipt-policy-id")}</dt>
                <dd translate="no">
                    <code data-testid="policy-id">
                        <strong data-testid="policy-short-id">{shortId}</strong>
                        {idDisplay.slice(shortId.length)}
                    </code>
                    {compare ? (
                        <Typography variant="body2" data-testid="policy-id-hint">
                            {t("receiptScreen.policy.compareGroups")}
                        </Typography>
                    ) : null}
                </dd>
                <dt>{words.t("receipt-event")}</dt>
                <dd>{summary.event}</dd>
                {witnesses ? (
                    <>
                        <dt>{words.t("receipt-witnesses")}</dt>
                        <dd>
                            <ul>
                                {summary.witnesses.map((w) => (
                                    <li key={w.name}>
                                        {w.name}
                                        {w.party
                                            ? `, ${words.t("receipt-witness-party", {party: w.party})}`
                                            : null}
                                    </li>
                                ))}
                            </ul>
                        </dd>
                    </>
                ) : null}
            </Facts>
        </Box>
    )
}
