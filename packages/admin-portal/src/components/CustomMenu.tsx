// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useState} from "react"
import {Menu, useSidebarState} from "react-admin"
import {faAngleDoubleLeft, faAngleDoubleRight} from "@fortawesome/free-solid-svg-icons"
import {IconButton, adminTheme} from "@sequentech/ui-essentials"
import {Box, Button, MenuItem, Typography, Menu as MMenu} from "@mui/material"
import {styled} from "@mui/material/styles"
import SelectTenants from "./menu/items/SelectTenants"
import ElectionEvents, {TREE_RESOURCE_NAMES} from "./menu/items/ElectionEvents"
import {useTranslation} from "react-i18next"
import GroupIcon from "@mui/icons-material/Group"
import SettingsIcon from "@mui/icons-material/Settings"
import HelpIcon from "@mui/icons-material/Help"
import MailIcon from "@mui/icons-material/Mail"
import {TenantContext} from "@/providers/TenantContextProvider"
import {IPermissions} from "@/types/keycloak"
import {AuthContext} from "@/providers/AuthContextProvider"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {useLocation, useNavigate} from "react-router"

const StyledHelpItem = styled(Button)`
    margin-top: -4px;
    margin-left: -1px;
    max-height: 36px;
    width: 100%;
    background-color: ${adminTheme.palette.white};
    color: ${adminTheme.palette.brandColor};
    border: 0px;
    border-radius: 0;

    &:hover {
        background-color: #f2f2f2;
        color: #333;
        box-shadow: none;
        border-radius: 0;
    }

    &:focus {
        outline: none;
        background-color: #ecfdf5;
        color: ${adminTheme.palette.brandColor};
        border-radius: 0;
        border: 0px;
    }
`

const StyledHelpItemContentWrapper = styled(Box)`
    display: flex;
    align-items: center;
    border: 0px solid red;
    width: 100%;
    flex-direction: row;
    justify-content: flex-start;
    gap: 15px;
    padding-left: 5px;
    flex: 1;
    max-height: 36px;
`

const StyledItem = styled(Menu.Item)`
    color: ${adminTheme.palette.brandColor};

    &.RaMenuItemLink-active,
    .MuiIconButton-root {
        color: ${adminTheme.palette.brandColor};
    }

    &.RaMenuItemLink-active {
        background-color: ${adminTheme.palette.green.light};
    }
`

const StyledMenu = styled(Menu)<{open: boolean}>`
    position: fixed;
    background-color: ${adminTheme.palette.white};
    color: ${adminTheme.palette.brandColor};
    margin-top: 0px;
    margin-right: 4px;
    box-shadow: 0px 2px 1px -1px rgba(0, 0, 0, 0.2), 0px 1px 1px 0px rgba(0, 0, 0, 0.14),
        0px 1px 3px 0px rgba(0, 0, 0, 0.12);
    border-radius: 4px;
    height: 100%;
    overflow-y: scroll;
    overflow-x: hidden;
    top: 80px;
    bottom: 36px;
    left: 0;
    scrollbar-width: ${({open}) => (open ? "thin" : "none")};
`

const DrawerContainer = styled(Box)<{open: boolean}>`
    position: fixed;
    bottom: 0;
    left: 0;
    background-color: ${adminTheme.palette.white};
    padding: 8px 16px;
    justify-content: right;
    border-top: 2px solid ${adminTheme.palette.customGrey.light};
    display: flex;
    margin-top: auto;
    width: ${({open}) => (open ? "300px" : "50px")};
`

const MenuWrapper = styled(Box)`
    border-bottom: 2px solid ${adminTheme.palette.customGrey.light};
    margin-bottom: 180px;
`

export const CustomMenu = () => {
    const {tenant} = useContext(TenantContext)
    const authContext = useContext(AuthContext)
    const {globalSettings} = useContext(SettingsContext)
    const [open, setOpen] = useSidebarState()
    const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null)
    const location = useLocation()
    const navigate = useNavigate()

    const {t, i18n} = useTranslation()

    const showUsers = authContext.isAuthorized(true, authContext.tenantId, IPermissions.USERS_MENU)
    const showSettings = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.SETTINGS_MENU
    )
    const showTemplates = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.TEMPLATES_MENU
    )

    const openInNewTab = (url: string) => {
        setAnchorEl(null)
        let replacedUrl = url.replace("${PUBLIC_BUCKET_URL}", globalSettings.PUBLIC_BUCKET_URL)
        window.open(replacedUrl, "_blank", "noopener,noreferrer")
    }

    const isElectionEventActive = TREE_RESOURCE_NAMES.some(
        (route) => location.pathname.search(route) > -1
    )

    /**
     * If route in TREE_RESOURCE_NAMES is active
     * and the user is either at the root
     * or a path with an empty third segment,
     * they are redirected to the root path (`/`).
     * This might be used to enforce navigation rules during an active election event
     */
    useEffect(() => {
        if (
            isElectionEventActive &&
            (location.pathname.split("/").length <= 2 ||
                (location.pathname.split("/").length > 2 && location.pathname.split("/")[2] === ""))
        ) {
            navigate("/")
        }
    }, [])

    return (
        <>
            <StyledMenu open={open}>
                <SelectTenants />

                <MenuWrapper>
                    <ElectionEvents />

                    {tenant && showUsers && (
                        <StyledItem
                            to="/user-roles"
                            primaryText={open ? t("sideMenu.usersAndRoles") : null}
                            leftIcon={<GroupIcon sx={{color: adminTheme.palette.brandColor}} />}
                        />
                    )}
                    {tenant && showSettings && (
                        <StyledItem
                            to="/settings"
                            primaryText={open ? t("sideMenu.settings") : null}
                            leftIcon={<SettingsIcon sx={{color: adminTheme.palette.brandColor}} />}
                        />
                    )}
                    {tenant && showTemplates && (
                        <StyledItem
                            to="/sequent_backend_template"
                            primaryText={open && t("sideMenu.templates")}
                            leftIcon={<MailIcon sx={{color: adminTheme.palette.brandColor}} />}
                        />
                    )}
                    {tenant?.settings?.help_links?.length > 0 && (
                        <StyledHelpItem
                            disableElevation
                            onClick={(e: React.MouseEvent<HTMLElement>) =>
                                setAnchorEl(e.currentTarget)
                            }
                        >
                            <StyledHelpItemContentWrapper>
                                <HelpIcon sx={{color: adminTheme.palette.brandColor}} />
                                <Typography>{t("sideMenu.help")}</Typography>
                            </StyledHelpItemContentWrapper>
                        </StyledHelpItem>
                    )}
                    {tenant?.settings?.help_links?.length > 0 && (
                        <MMenu
                            id="menu-sidebar"
                            anchorEl={anchorEl}
                            anchorOrigin={{
                                vertical: "bottom",
                                horizontal: "left",
                            }}
                            keepMounted
                            transformOrigin={{
                                vertical: "top",
                                horizontal: "right",
                            }}
                            open={Boolean(anchorEl)}
                            onClose={() => setAnchorEl(null)}
                        >
                            {tenant?.settings?.help_links?.map((i: any) => {
                                return (
                                    <MenuItem
                                        key={i.url}
                                        className="menu-sidebar-item"
                                        onClick={() => openInNewTab(i.url)}
                                    >
                                        <Box
                                            sx={{
                                                textOverflow: "ellipsis",
                                                whiteSpace: "nowrap",
                                                overflow: "hidden",
                                            }}
                                        >
                                            <span className="help-menu-item" title={i.title}>
                                                {i.i18n?.[i18n.language]?.title ?? i.title}
                                            </span>
                                        </Box>
                                    </MenuItem>
                                )
                            })}
                        </MMenu>
                    )}
                </MenuWrapper>

                <DrawerContainer open={open}>
                    <IconButton
                        icon={open ? faAngleDoubleLeft : faAngleDoubleRight}
                        fontSize="24px"
                        onClick={() => setOpen(!open)}
                    />
                </DrawerContainer>
            </StyledMenu>
        </>
    )
}
