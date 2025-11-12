// SPDX-FileCopyrightText: 2022-2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Box, Menu, MenuItem, Typography} from "@mui/material"
import React, {useEffect, useState} from "react"
import {useTranslation, Trans} from "react-i18next"
import {
    IExpiryCountdown,
    StyledButton,
    StyledButtonContainerWrapper,
    StyledButtonTooltip,
    UserProfile,
} from "../Header/Header"
import CountdownTimer from "../CountdownBar/CountdownBar"
import AccountCircle from "@mui/icons-material/AccountCircle"
import LogoutIcon from "@mui/icons-material/Logout"
import {styled} from "@mui/material/styles"
import theme from "../../services/theme"
import {EVotingPortalCountdownPolicy} from "@sequentech/ui-core"

const Span = styled("span")`
    font-size: 14px;
    color: ${theme.palette.customGrey.dark};
`

const Name = styled("span")`
    font-weight: 400;
`

export const StyledButtonTooltipText = styled(Typography)`
    padding: 0;
    margin: 0;
    font-size: 12px;
`

const CountdownTooltipContent: React.FC<{timeLeft?: string}> = ({timeLeft = ""}) => {
    const {t} = useTranslation()

    return (
        <>
            <StyledButtonTooltipText
                sx={{
                    fontWeight: 500,
                    color: theme.palette.brandColor,
                }}
            >
                {t("header.session.title")}
            </StyledButtonTooltipText>
            <StyledButtonTooltipText>
                {t("header.session.timeLeft", {time: timeLeft})}
            </StyledButtonTooltipText>
        </>
    )
}

interface ProfileMenuProps {
    userProfile: UserProfile
    logoutFn?: () => void
    setOpenModal: (value: boolean) => void
    handleOpenTimeModal: () => void
    expiry?: IExpiryCountdown
    setTimeLeftDialogText: (dialogText: string) => void
}

export const ProfileMenu: React.FC<ProfileMenuProps> = ({
    userProfile,
    logoutFn,
    setOpenModal,
    handleOpenTimeModal,
    expiry,
    setTimeLeftDialogText,
}) => {
    const {t} = useTranslation()
    const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null)
    const [timeLeftText, setTimeLeftText] = useState("")
    const [timeLeft, setTimeLeft] = useState(0)
    const [totalDuration, setTotalDuration] = useState(0)

    const [timeMinReached, setTimeMinReached] = useState(false)

    useEffect(() => {
        if (expiry && expiry.endTime) {
            const futureTime = expiry.endTime
            const timeLeftInSeconds = Math.floor((futureTime.getTime() - Date.now()) / 1000)
            setTimeLeft(timeLeftInSeconds)

            if (expiry.duration) {
                setTotalDuration(expiry.duration)
            } else if (expiry?.countdownAt && expiry?.countdownAt < timeLeftInSeconds) {
                setTotalDuration(expiry.countdownAt)
            } else {
                setTotalDuration(timeLeftInSeconds)
            }
        }
    }, [expiry])

    useEffect(() => {
        if (timeLeft > 0 && expiry?.countdown !== EVotingPortalCountdownPolicy.NO_COUNTDOWN) {
            if (
                expiry?.countdown === EVotingPortalCountdownPolicy.COUNTDOWN_WITH_ALERT &&
                expiry?.alertAt &&
                timeLeft < expiry?.alertAt &&
                !timeMinReached
            ) {
                setTimeMinReached(true)
                handleOpenTimeModal?.()
            }
            const timerId = setInterval(() => {
                /*if (timeLeft === 2) {
                    logoutFn?.() //TODO: still needs to better figure out how the access token vs refresh token is working
                }*/
                setTimeLeft(timeLeft - 1)
                if (timeLeft > 60) {
                    const timeLeftInMinutes: number = Math.floor(timeLeft / 60)
                    const time = timeLeft % 60
                    setTimeLeftText(
                        t("header.session.timeLeftMinutesAndSeconds", {time, timeLeftInMinutes})
                    )
                } else {
                    setTimeLeftText(t("header.session.timeLeftSeconds", {timeLeft}))
                }
            }, 1000)
            return () => clearInterval(timerId)
        }
    }, [expiry, timeLeft, handleOpenTimeModal, logoutFn, timeMinReached, t])

    useEffect(() => {
        setTimeLeftDialogText(timeLeftText)
    }, [timeLeftText, setTimeLeftDialogText])

    function handleMenu(event: React.MouseEvent<HTMLElement>) {
        setAnchorEl(event.currentTarget)
    }

    function handleClose() {
        setAnchorEl(null)
    }

    const profileName =
        userProfile.firstName && userProfile.firstName !== ""
            ? userProfile.firstName
            : userProfile.username

    return (
        <Box>
            <StyledButtonTooltip
                disableHoverListener={
                    !expiry || (expiry.countdownAt ? timeLeft > expiry?.countdownAt : true)
                }
                arrow
                placement="bottom-end"
                title={<CountdownTooltipContent timeLeft={timeLeftText} />}
                slotProps={{
                    popper: {
                        modifiers: [
                            {
                                name: "offset",
                                options: {
                                    offset: [-0, 10],
                                },
                            },
                        ],
                    },
                }}
            >
                <StyledButtonContainerWrapper className="logout-button-container-wrapper">
                    {expiry && timeLeft > 0 && timeLeft < totalDuration && (
                        <CountdownTimer progress={(timeLeft / totalDuration) * 100} />
                    )}
                    <StyledButton
                        className="logout-button"
                        aria-labelledby="welcome-text-name"
                        onClick={handleMenu}
                    >
                        <AccountCircle sx={{fontSize: 40}} />
                        <Box
                            id="welcome-text-name"
                            className="user-first-name"
                            sx={{
                                display: {xs: "none", sm: "block"},
                                maxWidth: "105px",
                                textOverflow: "ellipsis",
                                whiteSpace: "nowrap",
                                overflowX: "clip",
                                lineHeight: "18px",
                                fontWeight: "200",
                            }}
                            title={profileName}
                        >
                            <Trans
                                i18nKey="header.welcome"
                                values={{
                                    name: profileName,
                                }}
                                components={{br: <br />, span: <Name />}}
                            />
                        </Box>
                    </StyledButton>
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
                {(!!userProfile.firstName || !!userProfile.username || !!userProfile.email) && (
                    <MenuItem className="user-details">
                        <Box
                            sx={{
                                textOverflow: "ellipsis",
                                whiteSpace: "nowrap",
                                overflow: "hidden",
                            }}
                        >
                            {!!userProfile.username && (
                                <>
                                    <span
                                        className="firstname-or-username"
                                        title={
                                            userProfile.firstName && userProfile.firstName !== ""
                                                ? userProfile.firstName
                                                : userProfile.username
                                        }
                                    >
                                        {userProfile.firstName && userProfile.firstName !== ""
                                            ? userProfile.firstName
                                            : userProfile.username}
                                    </span>
                                    <br />
                                </>
                            )}
                            {!!userProfile.email && (
                                <Span className="email" title={userProfile.email}>
                                    {userProfile.email}
                                </Span>
                            )}
                        </Box>
                    </MenuItem>
                )}
                {userProfile.openLink && (
                    <MenuItem
                        className="profile"
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
}
