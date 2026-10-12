// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useId} from "react"
import {Box, Typography} from "@mui/material"
import {styled} from "@mui/material/styles"
import useMediaQuery from "@mui/material/useMediaQuery"
import {useTranslation} from "react-i18next"
import {faCheck, faCircleInfo, faUserGroup} from "@fortawesome/free-solid-svg-icons"
import {ICandidate, IContest, translate} from "@sequentech/ui-core"
import {Icon, theme, VisuallyHidden} from "@sequentech/ui-essentials"

import {getSlateName, IBallotSlates, IResolvedSlate, ISlateContest} from "../../services/Slates"
import {SlateCoverageChip, SlateCoverageSummary} from "./SlateCoverageLine"

const DESKTOP = theme.breakpoints.up("sm")
const PHONE = theme.breakpoints.down("sm")
const MAX_COLUMNS = 3
const LINE = "#e7eaee"

/** How much of a slate the ballot holds, which sets how its card is drawn. */
export enum ESlateCardState {
    NONE = "none",
    SOME = "some",
    EXACT = "exact",
}

/**
 * On desktop the cards share the rows of the list: one for the header, one
 * per office and one for the actions. A card takes those rows as its own, so
 * an office sits at the same height in every card of a row, even when a name
 * wraps. Below three slates the columns shrink to the number of slates; more
 * slates wrap onto new rows of three.
 */
const SlateList = styled("ul")`
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 12px;
    list-style: none;
    margin: 0;
    padding: 0;

    ${DESKTOP} {
        grid-template-columns: repeat(var(--slate-columns), minmax(0, 1fr));
        column-gap: 18px;
        row-gap: 0;
        align-items: stretch;
    }
`

const SlateCard = styled("li")`
    display: flex;
    flex-direction: column;
    min-width: 0;
    padding: 17px;
    border: 2px solid ${LINE};
    border-radius: 10px;
    background: ${theme.palette.white};
    overflow-wrap: anywhere;

    &.slate-card-some {
        border-color: #b5accf;
    }

    &.slate-card-exact {
        border-color: ${theme.palette.brandColor};
        background: #f9f9ff;
    }

    ${DESKTOP} {
        display: grid;
        grid-template-columns: minmax(0, 1fr);
        grid-template-rows: subgrid;
        row-gap: 0;
        margin-bottom: 18px;
        padding: 24px 21px 19px;
    }
`

const CardHeader = styled("div")`
    min-width: 0;
    padding-bottom: 7px;

    ${PHONE} {
        display: grid;
        grid-template-columns: minmax(0, 1fr) auto;
        column-gap: 10px;
        padding-bottom: 0;

        .slate-name {
            grid-column: 1;
            grid-row: 1;
        }

        .slate-symbol {
            grid-column: 2;
            grid-row: 1;
            align-self: start;
            margin: 0;
        }

        .slate-coverage-sentence,
        .slate-count,
        .slate-summary {
            grid-column: 1 / -1;
        }
    }
`

const Symbol = styled("div")`
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 10px;
    margin-bottom: 18px;
`

const Mark = styled("span")`
    width: 34px;
    height: 34px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: #efedf9;
    color: ${theme.palette.brandColor};
    font-size: 15px;

    ${PHONE} {
        display: none;
    }
`

const Office = styled("div")`
    min-width: 0;
    padding: 9px 0;
    border-bottom: 1px solid #edf0f5;

    &.slate-contest-last {
        border-bottom: none;
    }
`

const OfficeName = styled("h4")`
    margin: 0 0 5px;
    font-size: 12px;
    font-weight: 400;
    line-height: 1.4;
    color: ${theme.palette.customGrey.dark};
`

const NoCandidate = styled("p")`
    margin: 0;
    padding: 3px 0;
    font-size: 13px;
    line-height: 1.65;
    font-style: italic;
    color: #858b95;
`

const MemberList = styled("ul")`
    list-style: none;
    margin: 0;
    padding: 0;
    font-size: 14px;
    line-height: 1.5;
`

const MemberLists = styled("div")`
    ${DESKTOP} {
        display: contents;
    }
`

const Actions = styled("div")`
    display: flex;
    flex-direction: column;
    gap: 3px;
    margin-top: 8px;

    ${DESKTOP} {
        margin-top: 0;
        align-self: end;
    }

    .MuiButton-root {
        width: 100%;
    }
`

const Note = styled("p")`
    display: flex;
    align-items: flex-start;
    gap: 9px;
    margin: 20px 0 5px;
    font-size: 14px;
    line-height: 1.55;
    color: ${theme.palette.customGrey.dark};

    .slate-note-icon {
        margin-top: 3px;
    }

    ${PHONE} {
        margin-top: 16px;
        font-size: 12px;
        line-height: 1.5;
    }
`

const HEADER_ROWS = 1

export interface ISlateChooserProps {
    slates: IBallotSlates
    defaultLanguage?: string
    /** Replaces what the card says about the offices the slate covers. */
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
    /** How much of each slate the ballot holds. */
    getCardState?: (slate: IResolvedSlate) => ESlateCardState
}

/**
 * The slates of the ballot: each with its name, what it covers and its
 * candidates by office.
 */
export const SlateChooser: React.FC<ISlateChooserProps> = ({
    slates,
    defaultLanguage,
    renderCoverage,
    renderSummary,
    renderMemberLists,
    renderMember,
    renderActions,
    getCardState,
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
    const listStyle = {
        "--slate-columns": Math.min(MAX_COLUMNS, slates.slates.length),
    } as React.CSSProperties

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
        const isLast = isPhone
            ? slate.contests[slate.contests.length - 1]?.contest.id === contest.id
            : index === slates.contests.length - 1
        return (
            <Office
                key={contest.id}
                className={[
                    "slate-contest",
                    slateContest ? "" : "slate-contest-uncovered slate-contest-empty",
                    isLast ? "slate-contest-last" : "",
                ]
                    .filter(Boolean)
                    .join(" ")}
                data-contest-id={contest.id}
                style={officeRow(index)}
            >
                <OfficeName className="slate-contest-name">{contestName}</OfficeName>
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
            </Office>
        )
    }

    return (
        <Box component="section" className="slate-chooser" aria-labelledby={titleId}>
            <VisuallyHidden className="slate-chooser-title" id={titleId} component="h2">
                {t("slates.title")}
            </VisuallyHidden>
            <SlateList className="slate-list" style={listStyle}>
                {slates.slates.map((slate) => {
                    const slateName = getSlateName(slate, i18n.language, defaultLanguage)
                    const state = getCardState?.(slate) ?? ESlateCardState.NONE
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
                            className={`slate-card slate-card-${state}`}
                            data-slate-id={slate.id}
                            style={isPhone ? undefined : {gridRow: `span ${cardRows}`}}
                        >
                            <CardHeader className="slate-card-header">
                                <Symbol className="slate-symbol">
                                    <Mark className="slate-mark" aria-hidden="true">
                                        <Icon
                                            className="slate-mark-icon"
                                            icon={
                                                state === ESlateCardState.EXACT
                                                    ? faCheck
                                                    : faUserGroup
                                            }
                                        />
                                    </Mark>
                                    {slate.coverage ? (
                                        <SlateCoverageChip
                                            coverage={slate.coverage}
                                            contests={slate.contests.map((entry) => entry.contest)}
                                        />
                                    ) : null}
                                </Symbol>
                                <Typography
                                    className="slate-name"
                                    component="h3"
                                    fontSize="1.125rem"
                                    fontWeight="bold"
                                    lineHeight={1.3}
                                    margin={0}
                                >
                                    {slateName}
                                </Typography>
                                {renderCoverage ? (
                                    <Box className="slate-coverage">{renderCoverage(slate)}</Box>
                                ) : (
                                    <SlateCoverageSummary slate={slate} />
                                )}
                                {renderSummary ? (
                                    <Box className="slate-summary">{renderSummary(slate)}</Box>
                                ) : null}
                            </CardHeader>
                            {renderMemberLists ? renderMemberLists(slate, lists) : lists}
                            {renderActions ? (
                                <Actions
                                    className="slate-actions"
                                    style={isPhone ? undefined : {gridRow: actionsRow}}
                                >
                                    {renderActions(slate)}
                                </Actions>
                            ) : null}
                        </SlateCard>
                    )
                })}
            </SlateList>
            <Note className="slate-note">
                <Icon className="slate-note-icon" icon={faCircleInfo} aria-hidden="true" />
                <span className="slate-note-text">{t("slates.note")}</span>
            </Note>
        </Box>
    )
}
