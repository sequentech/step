// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// What the user compares with the official election notice before pinning a
// trust policy: its six words, its ID, its event and its witnesses, in the
// UI kit's own words. Everything here comes from the policy and is shown as
// text.

import React from "react"
import Box from "@mui/material/Box"
import Typography from "@mui/material/Typography"
import {styled} from "@mui/material/styles"
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
`

export interface IPolicyFactsProps {
    ui: TVitrineUi
    lang: Language
    summary: PolicySummary
    /** Show the heading and hint for the comparison with the notice. */
    compare?: boolean
    /** Show the witnesses. */
    witnesses?: boolean
}

export const PolicyFacts: React.FC<IPolicyFactsProps> = ({
    ui,
    lang,
    summary,
    compare = false,
    witnesses = false,
}) => {
    const words = ui.wordsFor(lang)
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
                <dt>{words.t("receipt-policy-words")}</dt>
                <dd lang={summary.lang} data-testid="policy-words">
                    {summary.words.join(" ")}
                </dd>
                <dt>{words.t("receipt-policy-id")}</dt>
                <dd>
                    <code data-testid="policy-id">{summary.id}</code>
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
