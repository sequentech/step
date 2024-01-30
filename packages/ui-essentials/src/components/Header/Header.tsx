// SPDX-FileCopyrightText: 2022-2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useRef, useState} from "react"

import Image from "mui-image"
import LanguageMenu from "../LanguageMenu/LanguageMenu"
import PageBanner from "../PageBanner/PageBanner"
import PageLimit from "../PageLimit/PageLimit"
import {theme} from "../../services/theme"
import LogoImg from "../../../public/Sequent_logo.svg"
import styled from "@emotion/styled"
import {Box, IconButton, Menu, MenuItem} from "@mui/material"
import Version from "../Version/Version"
import AccountCircle from "@mui/icons-material/AccountCircle"
import LogoutIcon from "@mui/icons-material/Logout"
import Dialog from "../Dialog/Dialog"
import {useTranslation} from "react-i18next"
import ProfileMenu from "../ProfileMenu/ProfileMenu"

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
    @media (max-width: ${theme.breakpoints.values.md}px) {
        height: 37px !important;
    }
`

type ApplicationVersion = {
    main: string
}

export type UserProfile = {
    username: string
    email?: string
    openLink: Function
}

export interface HeaderProps {
    logoutFn?: () => void
    appVersion?: ApplicationVersion
    logoLink?: string
    userProfile?: UserProfile
    logoUrl?: string
    languagesList?: Array<string>
    dir?: "ltr" | "rtl"
}

export default function Header({
    userProfile,
    appVersion,
    logoutFn,
    logoLink = "//sequentech.io/",
    logoUrl,
    languagesList,
    dir,
}: HeaderProps) {
    const {t} = useTranslation()
    // const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null)
    const [openModal, setOpenModal] = useState<boolean>(false)

    function handleCloseModal(value: boolean) {
        return value && logoutFn ? logoutFn() : setOpenModal(false)
    }

    // function handleMenu(event: React.MouseEvent<HTMLElement>) {
    //     setAnchorEl(event.currentTarget)
    // }

    // function handleClose() {
    //     setAnchorEl(null)
    // }

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
                            {userProfile ? (
                                <ProfileMenu
                                    userProfile={userProfile}
                                    openModalFn={setOpenModal}
                                    logoutFn={logoutFn}
                                    dir={dir}
                                />
                            ) : null}
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
