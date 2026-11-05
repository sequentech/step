// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// A notice in the Transparency tab when a hub of the event's policy lists a
// proven problem (DECISIONS D8). It is a convenience: it reads the hubs' lists
// through this portal, so it can only tell the truth while the portal does.
// It says so, and sends the admin to the hub's own site, where the proof is
// checked in the browser and which works even if this one lies. Silence from
// a hub is never shown as a problem.

import React, {useEffect, useState} from "react"
import {useTranslation} from "react-i18next"
import {Box, Typography} from "@mui/material"
import type {Policy} from "@vitrine/verify"
import {hubProblems, IHubProblems, problemKindKey, problemLink} from "@/services/HubProblems"
import {offeredHubs} from "@/services/HubUpdates"
import {BOARD} from "./TransparencyOverview"

const KEYS = "transparencyTab.hubProblems"

/** The problems the hubs of the policy list, asked once for each policy.
 * Undefined until they answer, and when there are no hubs to ask. */
export const useHubProblems = (policy: Policy | undefined): IHubProblems | undefined => {
    const [found, setFound] = useState<{policyId: string; problems: IHubProblems}>()
    const policyId = policy?.id

    useEffect(() => {
        if (!policy || policy.hubs.size === 0) return
        const controller = new AbortController()
        void hubProblems(policy, {signal: controller.signal}).then((problems) => {
            if (!controller.signal.aborted) setFound({policyId: policy.id, problems})
        })
        return () => controller.abort()
        // The policy object is rebuilt when the tab reads the event again; its
        // ID is what matters.
    }, [policyId])

    return found?.policyId === policyId ? found?.problems : undefined
}

export interface IHubProblemsBannerProps {
    policy: Policy | undefined
}

export const HubProblemsBanner: React.FC<IHubProblemsBannerProps> = ({policy}) => (
    <HubProblemsList policy={policy} found={useHubProblems(policy)} />
)

export interface IHubProblemsListProps {
    policy: Policy | undefined
    /** What `useHubProblems` found for the policy. */
    found: IHubProblems | undefined
}

/** The banner for problems already asked for, so that a page that also
 * counts them asks the hubs once. */
export const HubProblemsList: React.FC<IHubProblemsListProps> = ({policy, found}) => {
    const {t} = useTranslation()
    if (!policy || !found || found.problems.length === 0) return null
    const sites = new Map(offeredHubs(policy).map((hub) => [hub.name, hub.site]))
    const count = found.problems.length
    return (
        <Box
            role="status"
            data-testid="hub-problems"
            sx={{
                bgcolor: "#FFF7ED",
                color: "#9A3412",
                borderLeft: `3px solid ${BOARD.warning}`,
                p: "14px 16px",
                display: "flex",
                flexDirection: "column",
                gap: "10px",
            }}
        >
            <Typography
                component="h3"
                data-testid="hub-problems-title"
                sx={{fontSize: "15px", lineHeight: "22px", fontWeight: 700}}
            >
                {t(`${KEYS}.title`, {count})}
            </Typography>
            <Box component="ul" sx={{m: 0, pl: "18px", fontSize: "14px", lineHeight: "20px"}}>
                {found.problems.map((problem) => {
                    const link = problem.hubs
                        .map((hub) => problemLink(sites.get(hub), policy.id, problem.id))
                        .find(Boolean)
                    return (
                        <li key={problem.id} data-testid="hub-problem">
                            <b>{t(problemKindKey(problem.kind))}</b>
                            {" · "}
                            {t(`${KEYS}.listedBy`, {hubs: problem.hubs.join(", ")})}
                            {link ? (
                                <>
                                    {" · "}
                                    <Box
                                        component="a"
                                        href={link}
                                        target="_blank"
                                        rel="noopener noreferrer"
                                        sx={{color: "#9A3412", fontWeight: 600}}
                                    >
                                        {t(`${KEYS}.proof`)}
                                    </Box>
                                </>
                            ) : null}
                        </li>
                    )
                })}
            </Box>
            <Typography sx={{fontSize: "13px", lineHeight: "19px", color: BOARD.muted}}>
                {t(`${KEYS}.note`)}
            </Typography>
        </Box>
    )
}
