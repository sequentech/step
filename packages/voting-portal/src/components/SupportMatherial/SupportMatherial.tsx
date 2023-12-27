// SPDX-FileCopyrightText: 2022-2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box, Button, Typography} from "@mui/material"
import React from "react"
import {styled} from "@mui/material/styles"
import emotionStyled from "@emotion/styled"
import {FontAwesomeIcon} from "@fortawesome/react-fontawesome"
import {faTimes, faCheck} from "@fortawesome/free-solid-svg-icons"
// import {isUndefined} from "../../utils/typechecks"
import {useTranslation} from "react-i18next"
import {Dialog, theme} from "@sequentech/ui-essentials"
import VisibilityIcon from "@mui/icons-material/Visibility"

const BorderBox = styled(Box)`
    display: flex;
    flex-direction: row;
    border: 2px solid ${theme.palette.brandSuccess};
    background-color: ${theme.palette.lightBackground};
    display: flex;
    flex-direction: row;
    padding: 19px 38px;
    align-items: center;
    gap: 21px;
    color: ${({theme}) => theme.palette.black};

    @media (max-width: ${({theme}) => theme.breakpoints.values.md}px) {
        position: relative;
        flex-direction: column;
        padding: 27px 18px;
    }
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

const StatusBanner = styled(Box)`
    font-size: 14px;
    line-height: 20px;
    font-weight: 700;
    text-transform: uppercase;
    min-width: 85px;
    text-align: center;
    background-color: ${theme.palette.brandSuccess};
    color: ${theme.palette.black};
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
    font-size: 24px;
    line-height: 20px;
    margin-top: 0;
    margin-bottom: 10px;
    font-weight: bold;
    @media (max-width: ${({theme}) => theme.breakpoints.values.md}px) {
        margin-bottom: 0;
    }
`

const StyledSubTitle = styled(Typography)`
    font-size: 18px;
    line-height: 20px;
    margin-top: 0;
    margin-bottom: 10px;
    @media (max-width: ${({theme}) => theme.breakpoints.values.md}px) {
        margin-bottom: 0;
    }
`

export interface SupportMatherialProps {
    title: string
    subtitle?: string
    kind: string
    onClickToVote?: () => void
    onClickElectionResults?: () => void
    onClickBallotLocator?: () => void
}

export const SupportMatherial: React.FC<SupportMatherialProps> = ({
    title,
    subtitle,
    kind,
    onClickToVote,
    onClickElectionResults,
    onClickBallotLocator,
}) => {
    const {t} = useTranslation()
    const [openPreview, openPreviewSet] = React.useState<boolean>(false)

    console.log("title", title)
    console.log("subtitle", subtitle)
    console.log("kind", kind)

    // const handleClickToVote: React.MouseEventHandler<HTMLButtonElement | HTMLDivElement> = (
    //     event
    // ) => {
    //     event.stopPropagation()
    //     if (!isUndefined(onClickToVote)) {
    //         onClickToVote()
    //     }
    // }

    // const handleClickElectionResults: React.MouseEventHandler<HTMLButtonElement> = (event) => {
    //     event.stopPropagation()
    //     if (!isUndefined(onClickElectionResults)) {
    //         onClickElectionResults()
    //     }
    // }

    // const handleClickBallotLocator: React.MouseEventHandler<HTMLButtonElement | HTMLDivElement> = (
    //     event
    // ) => {
    //     event.stopPropagation()
    //     if (!isUndefined(onClickBallotLocator)) {
    //         onClickBallotLocator()
    //     }
    // }

    const handleOpenDialog = () => {
        openPreviewSet(true)
    }

    return (
        <>
            <BorderBox role="button" tabIndex={0}>
                <TextContainer>
                    <StyledTitle>{title}</StyledTitle>
                    <StyledSubTitle>{subtitle}</StyledSubTitle>
                </TextContainer>
                <Box sx={{display: "flex"}}>
                    <StyledButton sx={{marginRight: "16px"}} variant="secondary" onClick={handleOpenDialog}>
                        <VisibilityIcon />
                    </StyledButton>
                </Box>
            </BorderBox>

            <Dialog
                variant="info"
                open={openPreview}
                ok={t("tally.common.dialog.okCancel")}
                title={t("tally.common.dialog.cancelTitle")}
                handleClose={(result: boolean) => {
                    openPreviewSet(false)
                }}
                fullWidth
            >
                <Box sx={{display: "flex", flexDirection: "column", gap: "16px", width: "80vw", height: "100vh"}}>
                    <Typography variant="h4">{title}</Typography>
                    <Typography variant="body1">{subtitle}</Typography>
                </Box>
            </Dialog>
        </>
    )
}
