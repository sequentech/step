// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useId} from "react"
import {Box, Typography} from "@mui/material"
import {styled} from "@mui/material/styles"
import useMediaQuery from "@mui/material/useMediaQuery"
import {useTranslation} from "react-i18next"
import {ICandidate, IContest, translate} from "@sequentech/ui-core"
import {theme} from "@sequentech/ui-essentials"

import {getSlateName, IBallotSlates, IResolvedSlate, ISlateContest} from "../../services/Slates"
import {SlateCoverageLine} from "./SlateCoverageLine"

const DESKTOP = theme.breakpoints.up("sm")

const SlateList = styled("ul")`
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 16px;
    list-style: none;
    margin: 16px 0 24px;
    padding: 0;

    ${DESKTOP} {
        grid-template-columns: repeat(auto-fit, minmax(min(100%, 220px), 1fr));
    }
`

/**
 * On desktop a card spans one row of the list per part (header, each office,
 * actions) and takes those rows as its own, so a part is as tall as the
 * tallest one among the cards next to it and the offices line up.
 */
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

    .slate-actions {
        margin-top: auto;
    }

    ${DESKTOP} {
        display: grid;
        grid-template-columns: minmax(0, 1fr);
        grid-template-rows: subgrid;
        row-gap: 12px;

        .slate-card-header {
            display: flex;
            flex-direction: column;
        }

        .slate-summary {
            margin-top: auto;
        }

        .slate-actions {
            margin-top: 0;
        }
    }
`

const MemberLists = styled("div")`
    display: flex;
    flex-direction: column;
    gap: 12px;

    ${DESKTOP} {
        display: contents;
    }
`

const NoCandidate = styled("p")`
    margin: 4px 0 0;
    color: ${theme.palette.customGrey.dark};
    font-style: italic;
`

const HEADER_ROWS = 1

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
    /**
     * Wraps the lists of members of a slate, for example to collapse them. On
     * desktop the wrapper must not make a box of its own (`display: contents`),
     * or the offices stop lining up across cards.
     */
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
    const isPhone = useMediaQuery(theme.breakpoints.down("sm"))

    if (slates.slates.length === 0) {
        return null
    }

    const officeRow = (index: number): React.CSSProperties | undefined =>
        isPhone ? undefined : {gridRow: index + HEADER_ROWS + 1}
    const actionsRow = slates.contests.length + HEADER_ROWS + 1
    const cardRows = renderActions ? actionsRow : actionsRow - 1

    const renderOffice = (
        slate: IResolvedSlate,
        slateName: string,
        contest: IContest,
        index: number
    ) => {
        const slateContest = slate.contests.find((entry) => entry.contest.id === contest.id)
        if (!slateContest && isPhone) {
            return null
        }
        const contestName = translate(contest, "name", i18n.language) ?? ""
        return (
            <Box
                key={contest.id}
                className={
                    slateContest
                        ? "slate-contest"
                        : "slate-contest slate-contest-uncovered slate-contest-empty"
                }
                data-contest-id={contest.id}
                style={officeRow(index)}
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
                {slateContest ? (
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
                ) : (
                    <NoCandidate className="slate-no-candidate">
                        {t("slates.noCandidate")}
                    </NoCandidate>
                )}
            </Box>
        )
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
                    const lists = (
                        <MemberLists className="slate-member-lists">
                            {slates.contests.map((contest, index) =>
                                renderOffice(slate, slateName, contest, index)
                            )}
                        </MemberLists>
                    )

                    return (
                        <SlateCard
                            key={slate.id}
                            className="slate-card"
                            data-slate-id={slate.id}
                            style={isPhone ? undefined : {gridRow: `span ${cardRows}`}}
                        >
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
                                ) : slate.coverage ? (
                                    <SlateCoverageLine
                                        coverage={slate.coverage}
                                        contests={slate.contests.map((entry) => entry.contest)}
                                    />
                                ) : null}
                                {renderSummary ? (
                                    <Box className="slate-summary">{renderSummary(slate)}</Box>
                                ) : null}
                            </Box>
                            {renderMemberLists ? renderMemberLists(slate, lists) : lists}
                            {renderActions ? (
                                <Box
                                    className="slate-actions"
                                    style={isPhone ? undefined : {gridRow: actionsRow}}
                                >
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
