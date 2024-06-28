// SPDX-FileCopyrightText: 2022-2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"

import Image from "mui-image"
import LanguageMenu from "../LanguageMenu/LanguageMenu"
import PageBanner from "../PageBanner/PageBanner"
import PageLimit from "../PageLimit/PageLimit"
import {theme} from "../../services/theme"
import LogoImg from "../../../public/Sequent_logo.svg"
import styled from "@emotion/styled"
import {
    Box,
    Button,
    IconButton,
    Menu,
    MenuItem,
    Tooltip,
    TooltipProps,
    Typography,
    tooltipClasses,
} from "@mui/material"
import Version from "../Version/Version"
import AccountCircle from "@mui/icons-material/AccountCircle"
import LogoutIcon from "@mui/icons-material/Logout"
import Dialog from "../Dialog/Dialog"
import {useTranslation} from "react-i18next"
import CountdownTimer from "../CountdownBar/CountdownBar"

const HeaderWrapper = styled(PageBanner)`
    background-color: ${theme.palette.lightBackground};
    padding: 16px 0;
    font-size: 16px;

    @media (max-width: ${theme.breakpoints.values.lg}px) {
        padding: 9px;
    }
`

const Span = styled.span`
    font-size: 14px;
    color: ${theme.palette.customGrey.dark};
`

const StyledLink = styled.a`
    max-height: 100%;
    max-width: 50%;
`

const StyledImage = styled(Image)`
    height: 47px !important;
    width: unset !important;
    @media (max-width: ${theme.breakpoints.values.md}px) {
        height: 37px !important;
    }
`

const StyledButtonTooltip = styled(({className, ...props}: TooltipProps) => (
    <Tooltip {...props} classes={{popper: className}} />
))(({theme}) => ({
    [`& .${tooltipClasses.tooltip}`]: {
        backgroundColor: "#cce5ff",
        color: "rgba(0, 0, 0)",
        maxWidth: 220,
        fontSize: theme.typography.pxToRem(12),
        padding: 16,
        display: "flex",
        flexDirection: "column",
        gap: 8,
    },
    [`& .MuiTooltip-arrow`]: {
        color: "#cce5ff",
    },
}))

const StyledButtonTooltipText = styled(Typography)`
    padding: 0;
    margin: 0;
    font-size: 12px;
`

const StyledButtonContainerWrapper = styled.div`
    position: relative;
    padding: 0;
    margin: 0;
    width: 125px;
    height: 44px;
`

const StyledButtonContainer = styled.div`
    position: absolute;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: row;
    justify-content: center;
    align-items: center;
    height: 100%;
    width: 100%;
`

const StyledButton = styled(Button)`
    color: ${({theme}) => theme.palette.brandColor} !important;
    background: transparent !important;
    border: none;
    display: flex;
    width: 100%;
    // border-bottom: ${({theme}) => `2px solid ${theme.palette.brandColor}`} !important;
    outline: "none";
    box-sizing: "border-box";

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

type UserProfile = {
    username: string
    email?: string
    openLink?: Function
}

export enum HeaderErrorVariant {
    HIDE_PROFILE = "hide profile",
    SHOW_PROFILE = "show profile",
}

export interface HeaderProps {
    logoutFn?: () => void
    appVersion?: ApplicationVersion
    logoLink?: string
    userProfile?: UserProfile
    logoUrl?: string
    languagesList?: Array<string>
    errorVariant?: HeaderErrorVariant
}

export default function Header({
    userProfile,
    appVersion,
    logoutFn,
    logoLink,
    logoUrl,
    languagesList,
    errorVariant,
}: HeaderProps) {
    const {t} = useTranslation()
    const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null)
    const [openModal, setOpenModal] = useState<boolean>(false)

    function handleCloseModal(value: boolean) {
        return value && logoutFn ? logoutFn() : setOpenModal(false)
    }

    function handleMenu(event: React.MouseEvent<HTMLElement>) {
        setAnchorEl(event.currentTarget)
    }

    function handleClose() {
        setAnchorEl(null)
    }

    function timeContent() {
        return (
            <>
                <StyledButtonTooltipText
                    sx={{
                        fontWeight: 500,
                    }}
                >
                    Your session is going to expire.
                </StyledButtonTooltipText>
                <StyledButtonTooltipText>
                    You have 10 minutes left to cast your vote.
                </StyledButtonTooltipText>
            </>
        )
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
                            <StyledImage src={logoUrl ?? LogoImg} duration={100} alt="Logo Image" />
                        </StyledLink>
                        <Box
                            display="flex"
                            alignItems="center"
                            sx={{gap: {xs: "11px", lg: "31px"}}}
                        >
                            <Version version={appVersion ?? {main: "0.0.0"}} />
                            <LanguageMenu languagesList={languagesList} />
                            {errorVariant === HeaderErrorVariant.HIDE_PROFILE && !!logoutFn ? (
                                <StyledButtonTooltip
                                    arrow
                                    placement="bottom-end"
                                    title={timeContent()}
                                >
                                    <StyledButtonContainerWrapper>
                                        <StyledButtonContainer className="logout-button-container">
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
                                        </StyledButtonContainer>
                                        <CountdownTimer duration={5 * 60} />
                                    </StyledButtonContainerWrapper>
                                </StyledButtonTooltip>
                            ) : (
                                userProfile && (
                                    <Box>
                                        <StyledButtonTooltip
                                            arrow
                                            placement="bottom-end"
                                            title={timeContent()}
                                        >
                                            <StyledButtonContainerWrapper>
                                                <StyledButtonContainer className="logout-button-container">
                                                    <StyledButton
                                                        className="logout-button"
                                                        aria-label="log out button"
                                                        onClick={handleMenu}
                                                    >
                                                        <AccountCircle sx={{fontSize: 40}} />
                                                        <Box
                                                            sx={{
                                                                display: {xs: "none", sm: "block"},
                                                            }}
                                                        >
                                                            Time Left
                                                        </Box>
                                                    </StyledButton>
                                                </StyledButtonContainer>
                                                <CountdownTimer duration={5 * 60} />
                                            </StyledButtonContainerWrapper>
                                        </StyledButtonTooltip>
                                        <Menu
                                            id="menu-appbar"
                                            anchorEl={anchorEl}
                                            anchorOrigin={{
                                                vertical: "bottom",
                                                horizontal: "right",
                                            }}
                                            keepMounted
                                            transformOrigin={{
                                                vertical: "top",
                                                horizontal: "right",
                                            }}
                                            sx={{maxWidth: 220}}
                                            open={Boolean(anchorEl)}
                                            onClose={handleClose}
                                        >
                                            {(!!userProfile.username || !!userProfile.email) && (
                                                <MenuItem>
                                                    <Box
                                                        sx={{
                                                            textOverflow: "ellipsis",
                                                            whiteSpace: "nowrap",
                                                            overflow: "hidden",
                                                        }}
                                                    >
                                                        {!!userProfile.username && (
                                                            <>
                                                                <span title={userProfile.username}>
                                                                    {userProfile.username}
                                                                </span>
                                                                <br />
                                                            </>
                                                        )}
                                                        {!!userProfile.email && (
                                                            <Span title={userProfile.email}>
                                                                {userProfile.email}
                                                            </Span>
                                                        )}
                                                    </Box>
                                                </MenuItem>
                                            )}
                                            {userProfile.openLink && (
                                                <MenuItem
                                                    onClick={() => {
                                                        handleClose()
                                                        userProfile?.openLink?.()
                                                    }}
                                                >
                                                    <AccountCircle sx={{marginRight: "14px"}} />
                                                    {t("header.profile")}
                                                </MenuItem>
                                            )}
                                            {logoutFn && (
                                                <MenuItem
                                                    className="logout-button"
                                                    onClick={() => {
                                                        setOpenModal(true)
                                                        handleClose()
                                                    }}
                                                >
                                                    <LogoutIcon sx={{marginRight: "14px"}} />
                                                    {t("logout.buttonText")}
                                                </MenuItem>
                                            )}
                                        </Menu>
                                    </Box>
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
        </>
    )
}
