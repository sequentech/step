// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useId} from "react"
import {Box, Typography} from "@mui/material"
import {styled} from "@mui/material/styles"
import {useTranslation} from "react-i18next"
import {ICandidate, translate} from "@sequentech/ui-core"
import {theme} from "@sequentech/ui-essentials"

import {getSlateName, IBallotSlates, IResolvedSlate, ISlateContest} from "../../services/Slates"

const SlateList = styled("ul")`
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(100%, 240px), 1fr));
    gap: 16px;
    list-style: none;
    margin: 16px 0 24px;
    padding: 0;
`

const SlateCard = styled("li")`
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-width: 0;
    padding: 16px;
    border: 1px solid ${theme.palette.customGrey.light};
    border-radius: 4px;
    background: ${theme.palette.white};
    overflow-wrap: anywhere;
`

const MemberList = styled("ul")`
    list-style: none;
    margin: 4px 0 0;
    padding: 0;
`

export interface ISlateChooserProps {
    slates: IBallotSlates
    defaultLanguage?: string
    /** A line under the slate name saying what the slate covers. */
    renderCoverage?: (slate: IResolvedSlate) => React.ReactNode
    /** A line under the slate name saying how much of the slate is selected. */
    renderSummary?: (slate: IResolvedSlate) => React.ReactNode
    /** Wraps the lists of members of a slate, for example to collapse them. */
    renderMemberLists?: (slate: IResolvedSlate, lists: React.ReactNode) => React.ReactNode
    /** Replaces the row of one member; it must render an `li`. */
    renderMember?: (
        slate: IResolvedSlate,
        slateContest: ISlateContest,
        candidate: ICandidate
    ) => React.ReactNode
    /** What a voter can do with the slate. */
    renderActions?: (slate: IResolvedSlate) => React.ReactNode
}

/**
 * The slates of the ballot: each with its name and its candidates by contest.
 */
export const SlateChooser: React.FC<ISlateChooserProps> = ({
    slates,
    defaultLanguage,
    renderCoverage,
    renderSummary,
    renderMemberLists,
    renderMember,
    renderActions,
}) => {
    const {t, i18n} = useTranslation()
    const id = useId()
    const titleId = `${id}-title`

    if (slates.slates.length === 0) {
        return null
    }

    return (
        <Box component="section" className="slate-chooser" aria-labelledby={titleId}>
            <Typography
                className="slate-chooser-title"
                id={titleId}
                component="h2"
                fontSize="20px"
                fontWeight="bold"
                marginBottom="4px"
            >
                {t("slates.title")}
            </Typography>
            <Typography className="slate-chooser-description" color={theme.palette.customGrey.dark}>
                {t("slates.description")}
            </Typography>
            <SlateList className="slate-list">
                {slates.slates.map((slate) => {
                    const slateName = getSlateName(slate, i18n.language, defaultLanguage)
                    const lists = slate.contests.map((slateContest) => {
                        const contestName =
                            translate(slateContest.contest, "name", i18n.language) ?? ""
                        return (
                            <Box
                                key={slateContest.contest.id}
                                className="slate-contest"
                                data-contest-id={slateContest.contest.id}
                            >
                                <Typography
                                    className="slate-contest-name"
                                    component="h4"
                                    fontSize="14px"
                                    fontWeight="bold"
                                    margin={0}
                                >
                                    {contestName}
                                </Typography>
                                <MemberList
                                    className="slate-members"
                                    aria-label={t("slates.contestMembers", {
                                        slate: slateName,
                                        contest: contestName,
                                    })}
                                >
                                    {slateContest.candidates.map((candidate) =>
                                        renderMember ? (
                                            <React.Fragment key={candidate.id}>
                                                {renderMember(slate, slateContest, candidate)}
                                            </React.Fragment>
                                        ) : (
                                            <li
                                                key={candidate.id}
                                                className="slate-member"
                                                data-candidate-id={candidate.id}
                                            >
                                                {translate(candidate, "name", i18n.language)}
                                            </li>
                                        )
                                    )}
                                </MemberList>
                            </Box>
                        )
                    })

                    return (
                        <SlateCard key={slate.id} className="slate-card" data-slate-id={slate.id}>
                            <Box className="slate-card-header">
                                <Typography
                                    className="slate-name"
                                    component="h3"
                                    fontSize="18px"
                                    fontWeight="bold"
                                    margin={0}
                                >
                                    {slateName}
                                </Typography>
                                {renderCoverage ? (
                                    <Box className="slate-coverage">{renderCoverage(slate)}</Box>
                                ) : null}
                                {renderSummary ? (
                                    <Box className="slate-summary">{renderSummary(slate)}</Box>
                                ) : null}
                            </Box>
                            {renderMemberLists ? (
                                renderMemberLists(slate, lists)
                            ) : (
                                <Box
                                    className="slate-member-lists"
                                    display="flex"
                                    flexDirection="column"
                                    gap="12px"
                                >
                                    {lists}
                                </Box>
                            )}
                            {renderActions ? (
                                <Box className="slate-actions" marginTop="auto">
                                    {renderActions(slate)}
                                </Box>
                            ) : null}
                        </SlateCard>
                    )
                })}
            </SlateList>
        </Box>
    )
}
