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
    openDate?: string
    closeDate?: string
    onClickToVote?: () => void
    onClickElectionResults?: () => void
    onClickBallotLocator?: () => void
    electionDates?: IElectionDates
}

const SelectElection: React.FC<SelectElectionProps> = ({
    isActive,
    isOpen,
    title,
    electionHomeUrl,
    hasVoted,
    openDate,
    closeDate,
    onClickToVote,
    onClickElectionResults,
    onClickBallotLocator,
    electionDates,
}) => {
    const {t} = useTranslation()
    const timeLeft = useSelectElectionCountdown({date: electionDates?.start_date ?? ""})

    const handleClickToVote: React.MouseEventHandler<HTMLButtonElement | HTMLDivElement> = (
        event
    ) => {
        event.stopPropagation()

        if (!isUndefined(onClickToVote)) {
            onClickToVote()
        }
    }

    const handleClickElectionResults: React.MouseEventHandler<HTMLButtonElement> = (event) => {
        event.stopPropagation()
        if (!isUndefined(onClickElectionResults)) {
            onClickElectionResults()
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
                    <Box sx={{display: {xs: "none", md: "inline-flex"}}}>
                        <StyledLink href={electionHomeUrl} target="_blank">
                            {t("selectElection.electionWebsite")}
                        </StyledLink>
                    </Box>
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
                    {isOpen ? (
                        <StyledButton
                            className="click-to-vote-button"
                            disabled={!onClickToVote}
                            onClick={handleClickToVote}
                        >
                            {t("selectElection.voteButton")}
                        </StyledButton>
                    ) : (
                        <></>
                        // <StyledButton variant="secondary" onClick={handleClickElectionResults}>
                        //     {t("selectElection.resultsButton")}
                        // </StyledButton>
                    )}
                </Box>
            </BorderBox>
            {
                // Only show the countdown when there's a start date, the voting
                // period is not yet open, and the start date is in the future
                electionDates?.start_date &&
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
