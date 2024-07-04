import {Box, Menu, MenuItem} from "@mui/material"
import React, {useEffect, useState} from "react"
import {useTranslation} from "react-i18next"
import {
    StyledButton,
    StyledButtonContainer,
    StyledButtonContainerWrapper,
    StyledButtonTooltip,
} from "../Header/Header"
import CountdownTimer from "../CountdownBar/CountdownBar"
import AccountCircle from "@mui/icons-material/AccountCircle"
import LogoutIcon from "@mui/icons-material/Logout"
import styled from "@emotion/styled"
import theme from "../../services/theme"

const Span = styled.span`
    font-size: 14px;
    color: ${theme.palette.customGrey.dark};
`
export const ProfileMenu = ({
    CountdownTooltipContent,
    userProfile,
    logoutFn,
    setOpenModal,
    handleOpenTimeModal,
    expiry,
}) => {
    const {t} = useTranslation()

    const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null)
    const [timeLeftText, setTimeLeftText] = useState("")
    const [timeLeft, setTimeLeft] = useState(0)
    const [totalDuration, setTotalDuration] = useState(0)

    const [timeMinReached, setTimeMinReached] = useState(false)

    useEffect(() => {
        if (expiry) {
            const futureTime = expiry.endTime
            const currentTime = expiry.startTime
            const totalDuration: number =
                expiry.duration ?? Math.floor((futureTime.getTime() - currentTime.getTime()) / 1000)

            const timeLeftInSeconds = Math.floor((futureTime.getTime() - Date.now()) / 1000)

            console.log({timeLeftInSeconds, totalDuration})
            setTimeLeft(timeLeftInSeconds)
            setTotalDuration(60)
        }
    }, [])

    useEffect(() => {
        if (timeLeft > 0) {
            if (timeLeft < expiry.alertAt && !timeMinReached) {
                setTimeMinReached(true)
                handleOpenTimeModal?.()
            }

            const timerId = setInterval(() => {
                setTimeLeft(timeLeft - 1)
                if (timeLeft > 60) {
                    const timeLeftInMinutes: number = Math.floor(timeLeft / 60)
                    const time = timeLeft % 60
                    setTimeLeftText(`${timeLeftInMinutes} minutes and ${time} seconds`)
                } else {
                    setTimeLeftText(`${timeLeft} seconds`)
                }
            }, 1000)
            return () => clearInterval(timerId)
        }
    }, [expiry, timeLeft])

    function handleMenu(event: React.MouseEvent<HTMLElement>) {
        setAnchorEl(event.currentTarget)
    }

    function handleClose() {
        setAnchorEl(null)
    }

    return (
        <Box>
            <StyledButtonTooltip
                disableHoverListener={!expiry || timeLeft > 60}
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
                <StyledButtonContainerWrapper style={{width: 60}}>
                    <StyledButtonContainer className="logout-button-container">
                        <StyledButton
                            className="logout-button"
                            aria-label="log out button"
                            onClick={handleMenu}
                        >
                            <AccountCircle sx={{fontSize: 40}} />
                            {/* <Box
									sx={{
										display: {xs: "none", sm: "block"},
									}}
								>
									Profile
								</Box> */}
                        </StyledButton>
                    </StyledButtonContainer>
                    {expiry && timeLeft < 60 && (
                        <CountdownTimer progress={(timeLeft / totalDuration) * 100} />
                    )}
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
                                    <span title={userProfile.username}>{userProfile.username}</span>
                                    <br />
                                </>
                            )}
                            {!!userProfile.email && (
                                <Span title={userProfile.email}>{userProfile.email}</Span>
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
}
