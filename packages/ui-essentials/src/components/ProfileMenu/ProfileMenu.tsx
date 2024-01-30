// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"

import styled from "@emotion/styled"
import {theme} from "../../services/theme"
import {Box, IconButton, Menu, MenuItem} from "@mui/material"
import AccountCircle from "@mui/icons-material/AccountCircle"
import LogoutIcon from "@mui/icons-material/Logout"
import {useTranslation} from "react-i18next"

const Span = styled.span`
    font-size: 14px;
    color: ${theme.palette.customGrey.dark};
`

type UserProfile = {
    username: string
    email?: string
    openLink: Function
}

export interface ProfileMenuProps {
    logoutFn?: () => void
    userProfile?: UserProfile
    openModalFn: (value: boolean) => void
    dir?: "ltr" | "rtl"
}

const ProfileMenu: React.FC<ProfileMenuProps> = (props) => {
    const {logoutFn, userProfile, dir, openModalFn} = props

    const {t} = useTranslation()
    const [anchorEl, setAnchorEl] = React.useState<null | HTMLElement>(null)
    const open = Boolean(anchorEl)

    const handleClick = (event: React.MouseEvent<HTMLButtonElement>) => {
        setAnchorEl(event.currentTarget)
    }

    const handleClose = () => {
        setAnchorEl(null)
    }

    return (
        <Box>
            <IconButton
                id="profile-button"
                className="profile-menu-button"
                size="large"
                aria-label="account of current user"
                aria-controls={open ? "profile-menu" : undefined}
                aria-haspopup="true"
                aria-expanded={open ? "true" : undefined}
                onClick={handleClick}
                color="inherit"
            >
                <AccountCircle sx={{fontSize: 40}} />
            </IconButton>
            <Menu
                id="profile-menu"
                anchorEl={anchorEl}
                dir={dir}
                open={open}
                onClose={handleClose}
                MenuListProps={{
                    "aria-labelledby": "profile-menu",
                }}
            >
                <MenuItem>
                    <Box
                        sx={{
                            textOverflow: "ellipsis",
                            whiteSpace: "nowrap",
                            overflow: "hidden",
                            textAlign: "start",
                        }}
                    >
                        <span title={userProfile?.username}>{userProfile?.username}</span>
                        <br />
                        <Span title={userProfile?.email}>{userProfile?.email}</Span>
                    </Box>
                </MenuItem>
                <MenuItem
                    onClick={() => {
                        handleClose()
                        userProfile?.openLink()
                    }}
                >
                    <AccountCircle
                        sx={{
                            marginRight: dir === "rtl" ? "0" : "14px",
                            marginLeft: dir === "rtl" ? "14px" : "0",
                            textAlign: "start",
                        }}
                    />
                    {t("header.profile")}
                </MenuItem>
                {logoutFn && (
                    <MenuItem
                        className="logout-button"
                        onClick={() => {
                            openModalFn(true)
                            handleClose()
                        }}
                    >
                        <LogoutIcon
                            sx={{
                                marginRight: dir === "rtl" ? "0" : "14px",
                                marginLeft: dir === "rtl" ? "14px" : "0",
                                transform: dir === "rtl" ? "rotate(180deg)" : "rotate(0)",
                            }}
                        />
                        {t("logout.buttonText")}
                    </MenuItem>
                )}
            </Menu>
        </Box>
    )
}

export default ProfileMenu
