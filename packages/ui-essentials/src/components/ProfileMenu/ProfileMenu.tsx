import {Box, Menu, MenuItem} from "@mui/material"
import React, {useState} from "react"
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
    timeContent,
    userProfile,
    logoutFn,
    setOpenModal,
    handleOpenTimeModal,
}) => {
    const {t} = useTranslation()

    const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null)

    function handleMenu(event: React.MouseEvent<HTMLElement>) {
        setAnchorEl(event.currentTarget)
    }

    function handleClose() {
        setAnchorEl(null)
    }

    return (
        <Box>
            <StyledButtonTooltip
                arrow
                placement="bottom-end"
                title={timeContent()}
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
                    <CountdownTimer
                        duration={5 * 60}
                        onTimeMinReached={handleOpenTimeModal}
                        minTime={5 * 60}
                    />
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
