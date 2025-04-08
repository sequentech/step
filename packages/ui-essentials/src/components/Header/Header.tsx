// SPDX-FileCopyrightText: 2022-2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import Image from "mui-image"
import LanguageMenu from "../LanguageMenu/LanguageMenu"
import PageBanner from "../PageBanner/PageBanner"
import PageLimit from "../PageLimit/PageLimit"
import {theme} from "../../services/theme"
import styled from "@emotion/styled"
import {Box, Button, Tooltip, TooltipProps, tooltipClasses} from "@mui/material"
import Version from "../Version/Version"
import LogoutIcon from "@mui/icons-material/Logout"
import Dialog from "../Dialog/Dialog"
import {useTranslation} from "react-i18next"
import {ProfileMenu} from "../ProfileMenu/ProfileMenu"
import {EVotingPortalCountdownPolicy} from "@sequentech/ui-core"

const smBreakpoint = theme.breakpoints.values.sm

const HeaderWrapper = styled(PageBanner)`
    backgroundcolor: ${theme.palette.lightBackground};
    padding: 16px 0;
    fontsize: 16px;

    @media (max-width: ${theme.breakpoints.values.lg}px) {
        padding: 9px;
    }
`

const StyledLink = styled.a`
    maxheight: 100%;
    maxwidth: 50%;
`

const StyledImage = styled(Image)`
    height: 47px !important;
    width: unset !important;
    @media (max-width: ${theme.breakpoints.values.md}px) {
        height: 37px !important;
    }
    @media (max-width: ${smBreakpoint}px) {
        height: 30px !important;
    }
    @media (max-width: ${smBreakpoint / 2}px) {
        height: 20px !important;
    }
    @media (max-width: ${smBreakpoint / 3}px) {
        height: 10px !important;
    }
`

export const StyledButtonTooltip = styled(({className, ...props}: TooltipProps) => (
    <Tooltip {...props} classes={{popper: className}} />
))(({theme}) => ({
    [`& .${tooltipClasses.tooltip}`]: {
        backgroundColor: theme.palette.blue.light,
        color: "rgba(0, 0, 0)",
        width: 220,
        fontSize: theme.typography.pxToRem(12),
        padding: 16,
        display: "flex",
        flexDirection: "column",
        gap: 8,
    },
    [`& .MuiTooltip-arrow`]: {
        color: theme.palette.blue.light,
        fontSize: 20,
        transform: "translate3d(200px, 0px, 0px) !important",
    },
}))

export const StyledButtonContainerWrapper = styled.div`
    position: relative;
    display: inline-block;
    padding: 0;
    margin: 0;
`

export const StyledButton = styled(Button)`
    zindex: 1;
    position: relative;
    color: ${({theme}) => theme.palette.brandColor} !important;
    background: transparent !important;
    borderradius: 5px;
    border: none;
    display: flex;
    outline: "none";
    overflow: hidden;

    &:hover,
    &:focus,
    &:active {
        color: ${({theme}) => theme.palette.white} !important;
        background: ${({theme}) => theme.palette.brandColor} !important;
        boxshadow: none !important;
    }
`

type ApplicationVersion = {
    main: string
}

export type UserProfile = {
    firstName?: string
    username: string
    email?: string
    openLink?: Function
}

export enum HeaderErrorVariant {
    HIDE_PROFILE = "hide profile",
    SHOW_PROFILE = "show profile",
}

export interface IExpiryCountdown {
    endTime?: Date
    countdown?: EVotingPortalCountdownPolicy
    countdownAt?: number
    alertAt?: number
    duration?: number
}

export interface HeaderProps {
    logoutFn?: () => void
    appVersion?: ApplicationVersion
    appHash?: ApplicationVersion
    logoLink?: string
    userProfile?: UserProfile
    logoUrl?: string
    languagesList?: Array<string>
    errorVariant?: HeaderErrorVariant
    expiry?: IExpiryCountdown
}

export default function Header({
    userProfile,
    appVersion,
    appHash,
    logoutFn,
    logoLink,
    logoUrl,
    languagesList,
    errorVariant,
    expiry = undefined,
}: HeaderProps) {
    const {t} = useTranslation()
    const [openModal, setOpenModal] = useState<boolean>(false)
    const [openTimeModal, setOpenTimeModal] = useState<boolean>(false)
    const [countdownTimeLeft, setCountdownTimeLeft] = useState<string | undefined>(undefined)
    function handleCloseModal(value: boolean) {
        return value && logoutFn ? logoutFn() : setOpenModal(false)
    }

    function handleToggleTimeModal(value: boolean) {
        setOpenTimeModal(value)
    }

    return (
        <>
            <HeaderWrapper
                className="header-class"
                sx={{backgroundColor: theme.palette.lightBackground}}
            >
                <PageLimit maxWidth="lg" sx={{height: {xs: "37px", md: "47px"}}}>
                    <PageBanner direction="row" sx={{height: "100%"}}>
                        <StyledLink href={logoLink} target="_blank">
                            <StyledImage src={logoUrl || ""} duration={100} alt="Logo Image" />
                        </StyledLink>
                        <Box
                            display="flex"
                            alignItems="center"
                            sx={{gap: {xs: "11px", lg: "31px"}}}
                        >
                            <Version version={appVersion ?? {main: "0.0.0"}} />
                            <Version header="hash.header" version={appHash ?? {main: "-"}} />
                            <LanguageMenu languagesList={languagesList} />
                            {errorVariant === HeaderErrorVariant.HIDE_PROFILE && !!logoutFn ? (
                                <StyledButtonContainerWrapper className="logout-button-container-wrapper">
                                    <StyledButton
                                        className="logout-button"
                                        aria-label="log out button"
                                        onClick={() => {
                                            setOpenModal(true)
                                        }}
                                    >
                                        <LogoutIcon />
                                        <Box sx={{display: {xs: "none", sm: "block"}}}>
                                            {t("logout.buttonText")}
                                        </Box>
                                    </StyledButton>
                                </StyledButtonContainerWrapper>
                            ) : (
                                userProfile && (
                                    <ProfileMenu
                                        userProfile={userProfile}
                                        logoutFn={logoutFn}
                                        setOpenModal={setOpenModal}
                                        setTimeLeftDialogText={(timeLeft: string) =>
                                            setCountdownTimeLeft(timeLeft)
                                        }
                                        handleOpenTimeModal={() => handleToggleTimeModal(true)}
                                        expiry={expiry}
                                    />
                                )
                            )}
                        </Box>
                    </PageBanner>
                </PageLimit>
            </HeaderWrapper>

            <Dialog
                handleClose={handleCloseModal}
                open={openModal}
                title={t("logout.modal.title")}
                ok={t("logout.modal.ok")}
                cancel={t("logout.modal.close")}
                variant="action"
            >
                <p>{t("logout.modal.content")}</p>
            </Dialog>
            <Dialog
                handleClose={() => handleToggleTimeModal(false)}
                open={openTimeModal}
                title={t("header.session.title")}
                ok={t("logout.modal.ok")}
                cancel={t("logout.modal.close")}
                variant="info"
            >
                <p>{t("header.session.timeLeft", {time: countdownTimeLeft})}</p>
            </Dialog>
        </>
    )
}
