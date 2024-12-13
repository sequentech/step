// SPDX-FileCopyrightText: 2022-2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box, Button, Typography} from "@mui/material"
import React from "react"
import {styled} from "@mui/material/styles"
import emotionStyled from "@emotion/styled"
import {FontAwesomeIcon} from "@fortawesome/react-fontawesome"
import {faTimes, faCheck} from "@fortawesome/free-solid-svg-icons"
import theme from "../../services/theme"
import {IElectionDates, isUndefined} from "@sequentech/ui-core"
import {useTranslation} from "react-i18next"
import {useSelectElectionCountdown} from "./useSelectElectionCountdown"

const BorderBox = styled(Box)<{isopen: string; isactive: string}>`
    display: flex;
    flex-direction: row;
    border: 2px solid
        ${({isopen, theme}) =>
            "true" === isopen ? theme.palette.brandSuccess : theme.palette.customGrey.light};
    ${({isopen, theme}) =>
        "true" === isopen ? `background-color: ${theme.palette.lightBackground};` : ""}
    display: flex;
    flex-direction: row;
    padding: 19px 38px;
    align-items: center;
    gap: 21px;
    color: ${({theme}) => theme.palette.black};
    ${({isopen, theme}) =>
        "true" === isopen
            ? `
            &:hover {
                box-shadow: 0 5px 5px rgba(0,0,0,.5);
                background-color: ${theme.palette.customGrey.light};
            }
            &:focus {
                border: 2px solid ${theme.palette.brandColor};
            }
            &:active {
                background-color: unset;
            }
        `
            : ""}
    ${({isactive}) =>
        "true" === isactive
            ? `
            &:hover {
                cursor: pointer;
            }
        `
            : ""}
    @media (max-width: ${({theme}) => theme.breakpoints.values.md}px) {
        position: relative;
        flex-direction: column;
        padding: 27px 18px;
    }
`

const BannerBox = styled(Box)<{isopen: string; isactive: string}>`
    flex: 1;
    padding: 5px 5px;
    background: ${({isopen, theme}) =>
        "true" === isopen ? theme.palette.brandSuccess : theme.palette.customGrey.light};
    color: ${({theme}) => theme.palette.brandColor};
`

const TextContainer = styled(Box)`
    flex-grow: 2;
    text-align: left;
    @media (max-width: ${({theme}) => theme.breakpoints.values.md}px) {
        display: flex;
        flex-direction: row;
        justify-content: space-between;
        width: 100%;
    }
`

const StyledLink = emotionStyled.a`
    text-decoration: underline;
    font-weight: normal;
    display: flex;
    flex: direction: row;
    align-items: center;
    color: ${({theme}) => theme.palette.black};

    @media (max-width: ${({theme}) => theme.breakpoints.values.md}px) {
        justify-content: center;
    }
`

const VotedContainer = styled(Box)<{hasvoted: string}>`
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 4px;
    color: ${({hasvoted, theme}) =>
        "true" === hasvoted ? theme.palette.brandSuccess : theme.palette.errorColor};
`

const StatusBanner = styled(Box)<{isopen: string}>`
    font-size: 14px;
    line-height: 20px;
    font-weight: 700;
    text-transform: uppercase;
    min-width: 85px;
    text-align: center;
    background-color: ${({isopen, theme}) =>
        "true" === isopen ? theme.palette.brandSuccess : theme.palette.errorColor};
    color: ${({isopen, theme}) => ("true" === isopen ? theme.palette.black : theme.palette.white)};

    @media (max-width: ${({theme}) => theme.breakpoints.values.md}px) {
        position: absolute;
        top: 0;
        left: 0;
    }
`

const StyledButton = styled(Button)`
    padding: 10px 24px;
    min-width: unset;
`

const DatesContainer = styled(Box)`
    display: flex;
    flex-direction: column;
    margin-right: 35px;

    @media (max-width: ${({theme}) => theme.breakpoints.values.md}px) {
        flex-direction: row;
        gap: 20px;
        margin-right: 0;
    }
`

const DatesUrlWrap = styled(Box)`
    @media (max-width: ${({theme}) => theme.breakpoints.values.md}px) {
        order: 1;
    }
`

const StyledTitle = styled(Typography)`
    font-size: 18px;
    line-height: 20px;
    margin-top: 0;
    margin-bottom: 10px;
    font-weight: bold;

    @media (max-width: ${({theme}) => theme.breakpoints.values.md}px) {
        margin-bottom: 0;
    }
`

export interface SelectElectionProps {
    isActive: boolean // it could be active and closed in the demo/preview
    isOpen: boolean
    title: string
    electionHomeUrl?: string
    hasVoted: boolean
    onClickToVote?: () => void
    onClickBallotLocator?: () => void
    electionDates?: IElectionDates
}

/**
 * The algorithm for election start date in voting portal's election list should
 * be:
 *
 * 1. If there's a scheduled event for start voting period, use that date
 * 2. Or else, if there's a scheduled event for allow initialization report, use
 *    that date
 * 3. Or else, if the election has been started, use that execution date (field
 *    `StringifiedPeriodDates::first_started_at`)
 *
 * The previously mentioned start-date should be applied in relation to:
 * - the shown start date related to the election
 * - the countdown to start
 *
 * The rationale for prioritizing the scheduled dates instead of the actual
 * execution dates is  * so that the dates don't change for voters.
 * */
const getStartDate = (electionDates?: IElectionDates): string | null => {
    return (
        electionDates?.scheduled_event_dates?.START_VOTING_PERIOD?.scheduled_at ||
        electionDates?.scheduled_event_dates?.ALLOW_INIT_REPORT?.scheduled_at ||
        electionDates?.first_started_at ||
        null
    )
}

/**
 *
 * The algorithm for the election end date in voting portal's election list
 * should be:
 *
 * 1. if there's a scheduled event for end voting period, use that date
 * 2. or else, if there's a scheduled event for allow end voting period, use
 *    that date
 * 3. or else, if the election has been stopped, use the execution date (field
 *    `StringifiedPeriodDates::last_stopped_at`)
 */
const getEndDate = (electionDates?: IElectionDates): string | null => {
    return (
        electionDates?.scheduled_event_dates?.END_VOTING_PERIOD?.scheduled_at ||
        electionDates?.scheduled_event_dates?.ALLOW_VOTING_PERIOD_END?.scheduled_at ||
        electionDates?.last_stopped_at ||
        null
    )
}

const formatDate = (input: string): string => {
    const dateFormatter = new Intl.DateTimeFormat("en-GB", {
        year: "numeric",
        month: "2-digit",
        day: "2-digit",
        hour: "2-digit",
        minute: "2-digit",
        hour12: false, // Specify 24-hour format
    })
    let date = new Date(input)
    return dateFormatter.format(date)
}

const hasDate = (date: string) => date.length > 0 && date !== "-"

const SelectElection: React.FC<SelectElectionProps> = ({
    isActive,
    isOpen,
    title,
    electionHomeUrl,
    hasVoted,
    onClickToVote,
    onClickBallotLocator,
    electionDates,
}) => {
    const {t} = useTranslation()
    const startVotingDate = getStartDate(electionDates) ?? ""
    const endVotingDate = getEndDate(electionDates) ?? ""
    const openDate = hasDate(startVotingDate) && formatDate(startVotingDate)
    const closeDate = hasDate(endVotingDate) && formatDate(endVotingDate)
    const timeLeft = useSelectElectionCountdown({date: startVotingDate ?? ""})

    const handleClickToVote: React.MouseEventHandler<HTMLButtonElement | HTMLDivElement> = (
        event
    ) => {
        event.stopPropagation()

        if (!isUndefined(onClickToVote)) {
            onClickToVote()
        }
    }

    const handleClickBallotLocator: React.MouseEventHandler<HTMLButtonElement | HTMLDivElement> = (
        event
    ) => {
        event.stopPropagation()
        if (!isUndefined(onClickBallotLocator)) {
            onClickBallotLocator()
        }
    }

    const displayBallotLocator = !!onClickBallotLocator

    return (
        <Box>
            <BorderBox
                onClick={handleClickToVote}
                isopen={String(!!isOpen)}
                isactive={String(!!isActive)}
                role="button"
                tabIndex={0}
                className="election-item"
            >
                <TextContainer className="election-info">
                    <StyledTitle className="election-title">{title}</StyledTitle>
                    {electionHomeUrl && (
                        <Box sx={{display: {xs: "none", md: "inline-flex"}}}>
                            <StyledLink href={electionHomeUrl} target="_blank">
                                {t("selectElection.electionWebsite")}
                            </StyledLink>
                        </Box>
                    )}
                    {hasVoted ? (
                        <VotedContainer
                            hasvoted={String(!!hasVoted)}
                            color={theme.palette.errorColor}
                        >
                            <FontAwesomeIcon icon={faCheck} size="sm" />
                            <Typography fontSize="14px" margin={0}>
                                {t("selectElection.voted")}
                            </Typography>
                        </VotedContainer>
                    ) : (
                        <VotedContainer
                            hasvoted={String(!!hasVoted)}
                            color={theme.palette.brandSuccess}
                        >
                            <FontAwesomeIcon icon={faTimes} size="sm" />
                            <Typography fontSize="14px" margin={0}>
                                {t("selectElection.notVoted")}
                            </Typography>
                        </VotedContainer>
                    )}
                </TextContainer>
                <StatusBanner isopen={String(!!isOpen)}>
                    {t(`selectElection.${isOpen ? "openElection" : "closedElection"}`)}
                </StatusBanner>
                <DatesUrlWrap>
                    <DatesContainer>
                        <Typography fontSize="16px" lineHeight="23px" margin={0}>
                            {t("selectElection.openDate")}
                            <b>{openDate || "-"}</b>
                        </Typography>
                        <Typography fontSize="16px" lineHeight="23px" margin={0}>
                            {t("selectElection.closeDate")}
                            <b>{closeDate || "-"}</b>
                        </Typography>
                    </DatesContainer>
                    <Box sx={{display: {xs: "block", md: "none"}}}>
                        <StyledLink href={electionHomeUrl} target="_blank">
                            {t("selectElection.electionWebsite")}
                        </StyledLink>
                    </Box>
                </DatesUrlWrap>
                <Box sx={{display: "flex"}} className="election-actions">
                    {displayBallotLocator && (
                        <StyledButton
                            sx={{marginRight: "16px"}}
                            variant="secondary"
                            onClick={handleClickBallotLocator}
                        >
                            {t("selectElection.ballotLocator")}
                        </StyledButton>
                    )}
                    {isOpen && (
                        <StyledButton
                            className="click-to-vote-button"
                            disabled={!onClickToVote}
                            onClick={handleClickToVote}
                        >
                            {t("selectElection.voteButton")}
                        </StyledButton>
                    )}
                </Box>
            </BorderBox>
            {
                // Only show the countdown when there's a start date, the voting
                // period is not yet open, and the start date is in the future
                getStartDate(electionDates) &&
                    !isOpen &&
                    timeLeft?.totalSeconds &&
                    timeLeft?.totalSeconds > 0 && (
                        <BannerBox
                            id="countdown-banner-box"
                            isopen={String(!!isOpen)}
                            isactive={String(!!isActive)}
                        >
                            <Typography sx={{margin: 0}}>
                                {t("selectElection.countdown", {
                                    years: timeLeft.years,
                                    months: timeLeft.months,
                                    weeks: timeLeft.weeks,
                                    days: timeLeft.days,
                                    hours: timeLeft.hours,
                                    minutes: timeLeft.minutes,
                                    seconds: timeLeft.seconds,
                                })}
                            </Typography>
                        </BannerBox>
                    )
            }
        </Box>
    )
}

export default SelectElection
