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
import {Box, IconButton, Menu, MenuItem} from "@mui/material"
import Version from "../Version/Version"
import AccountCircle from "@mui/icons-material/AccountCircle"
import LogoutIcon from "@mui/icons-material/Logout"
import Dialog from "../Dialog/Dialog"
import {useTranslation} from "react-i18next"

const HeaderWrapper = styled(PageBanner)`
    background-color: ${theme.palette.lightBackground};
    padding: 16px 0;
    font-size: 16px;

    @media (max-width: ${theme.breakpoints.values.lg}px) {
        padding: 9px;
    }
`

const StyledLink = styled.a`
    max-height: 100%;
    max-width: 50%;
`

const StyledImage = styled(Image)`
    max-height: 100%;
`

type ApplicationVersion = {
    main: string
}

export interface HeaderProps {
    logoutFn?: () => void
    version?: ApplicationVersion
    logoLink?: string
}

export default function Header({
    version: appVersion,
    logoutFn,
    logoLink = "//sequentech.io/",
}: HeaderProps) {
    const {t} = useTranslation()
    const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null)
    const [openModal, setOpenModal] = useState<boolean>(false)

    function handleCloseModal(value: boolean) {
        return value && logoutFn ? logoutFn() : setOpenModal(false)
    }

    function handleChange(event: React.ChangeEvent<HTMLInputElement>) {
        console.log("toto")
    }

    function handleMenu(event: React.MouseEvent<HTMLElement>) {
        setAnchorEl(event.currentTarget)
    }

    function handleClose() {
        setAnchorEl(null)
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
                            <StyledImage src={LogoImg} duration={100} alt="Logo Image" />
                        </StyledLink>
                        <Box
                            display="flex"
                            alignItems="center"
                            sx={{gap: {xs: "11px", lg: "31px"}}}
                        >
                            <Version version={appVersion ?? {main: "7.1.0"}} />
                            <LanguageMenu />
                            <div>
                                <IconButton
                                    size="large"
                                    aria-label="account of current user"
                                    aria-controls="menu-appbar"
                                    aria-haspopup="true"
                                    onClick={handleMenu}
                                    color="inherit"
                                >
                                    <AccountCircle />
                                </IconButton>
                                <Menu
                                    id="menu-appbar"
                                    anchorEl={anchorEl}
                                    anchorOrigin={{
                                        vertical: "top",
                                        horizontal: "right",
                                    }}
                                    keepMounted
                                    transformOrigin={{
                                        vertical: "top",
                                        horizontal: "right",
                                    }}
                                    open={Boolean(anchorEl)}
                                    onClose={handleClose}
                                >
                                    <MenuItem>
                                        <Box>
                                            <p>
                                                Name
                                                <br />
                                                eee@eee.com
                                            </p>
                                        </Box>
                                    </MenuItem>
                                    <MenuItem onClick={handleClose}>
                                        <AccountCircle sx={{marginRight: "14px"}} />
                                        Profile
                                    </MenuItem>
                                    {logoutFn && (
                                        <MenuItem
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
                            </div>
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
