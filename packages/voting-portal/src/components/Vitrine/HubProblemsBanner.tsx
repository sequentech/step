// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// A notice in the ballot status dialog when a hub of the election's policy
// lists a proven problem (DECISIONS D8). It is a convenience: it reads the
// hubs' lists through this portal, so it can only tell the truth while the
// portal does. It says so, and sends the voter to the hub's own site, where
// the proof is checked in the browser, and which works even if this one lies.

import React from "react"
import {Box} from "@mui/material"
import {useTranslation} from "react-i18next"
import type {Policy} from "@vitrine/verify"
import {useHubProblems} from "../../hooks/useHubProblems"
import {problemKindKey, problemLink} from "../../services/vitrine/hubProblems"
import {offeredHubs} from "../../services/vitrine/hubUpdates"

const COLORS = {
    redText: "#7F1D1D",
    redFill: "#FEF2F2",
    redLine: "#FECACA",
    redStrong: "#B91C1C",
    quiet: "#64748B",
} as const

export interface IHubProblemsBannerProps {
    policy: Policy | undefined
    /** Whether to ask the hubs: while the dialog is open. */
    enabled?: boolean
}

export const HubProblemsBanner: React.FC<IHubProblemsBannerProps> = ({policy, enabled = true}) => {
    const {t} = useTranslation()
    const found = useHubProblems(policy, enabled)
    if (!policy || !found || found.problems.length === 0) return null
    const sites = new Map(offeredHubs(policy).map((hub) => [hub.name, hub.site]))
    const count = found.problems.length
    return (
        <Box
            className="hub-problems-banner"
            role="status"
            sx={{
                border: `1px solid ${COLORS.redLine}`,
                bgcolor: COLORS.redFill,
                color: COLORS.redText,
                borderRadius: "4px",
                p: "14px",
                display: "flex",
                flexDirection: "column",
                gap: "10px",
                fontSize: "14px",
                lineHeight: "20px",
            }}
        >
            <Box className="hub-problems-title" sx={{fontWeight: 700}}>
                {t(
                    count === 1
                        ? "vitrine.hubProblems.title.one"
                        : "vitrine.hubProblems.title.other",
                    {
                        count,
                    }
                )}
            </Box>
            <Box component="ul" className="hub-problems-list" sx={{m: 0, pl: "18px"}}>
                {found.problems.map((problem) => {
                    const site = problem.hubs.map((hub) => sites.get(hub)).find(Boolean)
                    return (
                        <li key={problem.id} className="hub-problem">
                            <b className="hub-problem-kind">{t(problemKindKey(problem.kind))}</b>
                            {" · "}
                            <span className="hub-problem-hubs">
                                {t("vitrine.hubProblems.listedBy", {hubs: problem.hubs.join(", ")})}
                            </span>
                            {site ? (
                                <>
                                    {" · "}
                                    <Box
                                        component="a"
                                        className="hub-problem-link"
                                        href={problemLink(site, policy.id, problem.id)}
                                        target="_blank"
                                        rel="noopener noreferrer"
                                        sx={{color: COLORS.redStrong, fontWeight: 500}}
                                    >
                                        {t("vitrine.hubProblems.proof")}
                                    </Box>
                                </>
                            ) : null}
                        </li>
                    )
                })}
            </Box>
            <Box className="hub-problems-note" sx={{fontSize: "13px", color: COLORS.quiet}}>
                {t("vitrine.hubProblems.note")}
            </Box>
        </Box>
    )
}
