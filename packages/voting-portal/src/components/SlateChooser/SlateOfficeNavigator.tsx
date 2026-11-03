// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useId} from "react"
import {Box, Button} from "@mui/material"
import {styled} from "@mui/material/styles"
import useMediaQuery from "@mui/material/useMediaQuery"
import {faAngleLeft, faAngleRight, faCheck} from "@fortawesome/free-solid-svg-icons"
import {BallotSelection, IContest, translate} from "@sequentech/ui-core"
import {Icon, theme} from "@sequentech/ui-essentials"
import {useTranslation} from "react-i18next"

import {getContestSelectionCount} from "../../services/ReviewSummary"

const Layout = styled("div")`
    display: grid;
    grid-template-columns: 237px minmax(0, 1fr);
    gap: 35px;

    ${theme.breakpoints.down("sm")} {
        display: block;
    }
`

const OfficeList = styled("nav")`
    padding-right: 20px;
    border-right: 1px solid #e7eaee;

    ul {
        list-style: none;
        margin: 0;
        padding: 0;
    }
`

const OfficeButton = styled("button")`
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    min-height: 44px;
    margin-bottom: 5px;
    padding: 13px 12px;
    border: none;
    border-radius: 4px;
    background: ${theme.palette.white};
    color: ${theme.palette.customGrey.dark};
    font: inherit;
    font-size: 14px;
    text-align: start;
    overflow-wrap: anywhere;
    cursor: pointer;

    &[aria-current="true"] {
        background: #efedf9;
        color: ${theme.palette.brandColor};
        font-weight: 600;
    }

    &:focus-visible {
        outline: 3px solid ${theme.palette.brandColor};
        outline-offset: 2px;
    }

    .slate-office-name {
        flex: 1;
        min-width: 0;
    }

    .slate-office-count {
        font-size: 12px;
        white-space: nowrap;
    }

    .slate-office-full {
        color: #047857;
    }
`

const Picker = styled("div")`
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 24px;

    label {
        font-size: 13px;
        color: ${theme.palette.customGrey.dark};
        white-space: nowrap;
    }

    select {
        width: 100%;
        min-width: 0;
        height: 45px;
        padding: 9px 10px;
        border: 1px solid #c6c9d3;
        border-radius: 5px;
        background: ${theme.palette.white};
        color: ${theme.palette.brandColor};
        font: inherit;
        font-size: 14px;
    }
`

const OfficeIndex = styled("p")`
    margin: 0;
    font-size: 12px;
    color: ${theme.palette.customGrey.dark};
    text-transform: uppercase;

    ${theme.breakpoints.down("sm")} {
        font-size: 11px;
    }
`

const Navigation = styled("div")`
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    margin-top: 20px;
    padding-top: 15px;
    border-top: 1px solid #e7eaee;

    .slate-office-last {
        font-size: 13px;
        color: ${theme.palette.customGrey.dark};
    }
`

export interface SlateOfficeNavigatorProps {
    /** The offices of the ballot, in ballot order. */
    contests: Array<IContest>
    selection: BallotSelection | undefined
    /** The index of the office shown. */
    office: number
    onOfficeChange: (office: number) => void
    renderContest: (contest: IContest) => React.ReactNode
}

/**
 * The individual candidates of a ballot with slates, one office at a time.
 * Every office stays mounted, hidden when it is not shown, so the checks that
 * run before review still see all of them.
 */
export const SlateOfficeNavigator: React.FC<SlateOfficeNavigatorProps> = ({
    contests,
    selection,
    office,
    onOfficeChange,
    renderContest,
}) => {
    const {t, i18n} = useTranslation()
    const isPhone = useMediaQuery(theme.breakpoints.down("sm"))
    const pickerId = useId()
    const current = Math.min(Math.max(office, 0), contests.length - 1)

    const nameOf = (contest: IContest) => translate(contest, "name", i18n.language) ?? contest.id
    const countOf = (contest: IContest) =>
        t("slates.offices.count", {
            selected: getContestSelectionCount(contest, selection),
            max: contest.max_votes,
        })

    return (
        <Layout className="slate-office-navigator">
            {isPhone ? null : (
                <OfficeList className="slate-office-list" aria-label={t("slates.offices.label")}>
                    <ul className="slate-office-items">
                        {contests.map((contest, index) => (
                            <li key={contest.id} className="slate-office-item">
                                <OfficeButton
                                    type="button"
                                    className="slate-office-button"
                                    aria-current={index === current ? "true" : undefined}
                                    onClick={() => onOfficeChange(index)}
                                >
                                    <span className="slate-office-name">{nameOf(contest)}</span>
                                    <span className="slate-office-count">{countOf(contest)}</span>
                                    {getContestSelectionCount(contest, selection) >=
                                    contest.max_votes ? (
                                        <Icon
                                            className="slate-office-full"
                                            icon={faCheck}
                                            aria-hidden="true"
                                        />
                                    ) : null}
                                </OfficeButton>
                            </li>
                        ))}
                    </ul>
                </OfficeList>
            )}
            <Box className="slate-office-panel" sx={{minWidth: 0}}>
                {isPhone ? (
                    <Picker className="slate-office-picker">
                        <label className="slate-office-picker-label" htmlFor={pickerId}>
                            {t("slates.offices.picker")}
                        </label>
                        <select
                            className="slate-office-select"
                            id={pickerId}
                            value={current}
                            onChange={(event) => onOfficeChange(Number(event.target.value))}
                        >
                            {contests.map((contest, index) => (
                                <option
                                    key={contest.id}
                                    className="slate-office-option"
                                    value={index}
                                >
                                    {`${nameOf(contest)} · ${countOf(contest)}`}
                                </option>
                            ))}
                        </select>
                    </Picker>
                ) : null}
                <OfficeIndex className="slate-office-index">
                    {t("slates.offices.index", {index: current + 1, total: contests.length})}
                </OfficeIndex>
                {contests.map((contest, index) => (
                    <div
                        key={contest.id}
                        className="slate-office-contest"
                        data-contest-id={contest.id}
                        hidden={index !== current}
                    >
                        {renderContest(contest)}
                    </div>
                ))}
                <Navigation className="slate-office-navigation">
                    <Button
                        className="slate-office-previous"
                        variant="secondary"
                        disabled={current === 0}
                        onClick={() => onOfficeChange(current - 1)}
                        startIcon={
                            <Icon
                                className="slate-office-previous-icon"
                                icon={faAngleLeft}
                                aria-hidden="true"
                            />
                        }
                    >
                        {t("slates.offices.previous")}
                    </Button>
                    {current < contests.length - 1 ? (
                        <Button
                            className="slate-office-next"
                            variant="secondary"
                            onClick={() => onOfficeChange(current + 1)}
                            endIcon={
                                <Icon
                                    className="slate-office-next-icon"
                                    icon={faAngleRight}
                                    aria-hidden="true"
                                />
                            }
                        >
                            {t("slates.offices.next")}
                        </Button>
                    ) : (
                        <span className="slate-office-last">{t("slates.offices.last")}</span>
                    )}
                </Navigation>
            </Box>
        </Layout>
    )
}
