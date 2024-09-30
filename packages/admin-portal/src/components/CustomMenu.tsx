// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Menu, useSidebarState} from "react-admin"
import {faAngleDoubleLeft, faAngleDoubleRight} from "@fortawesome/free-solid-svg-icons"
import {IconButton, adminTheme} from "@sequentech/ui-essentials"
import {Box} from "@mui/material"
import {styled} from "@mui/material/styles"
import SelectTenants from "./menu/items/SelectTenants"
import ElectionEvents from "./menu/items/ElectionEvents"
import {useTranslation} from "react-i18next"
import GroupIcon from "@mui/icons-material/Group"
import SettingsIcon from "@mui/icons-material/Settings"
import MailIcon from "@mui/icons-material/Mail"

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
    margin-bottom: 116px;
`

export const CustomMenu = () => {
    const [open, setOpen] = useSidebarState()
    const {t} = useTranslation()

    return (
        <>
            <StyledMenu open={open}>
                <MenuWrapper>
                    <SelectTenants />

                    <ElectionEvents />

                    <StyledItem
                        to="/user-roles"
                        primaryText={open ? t("sideMenu.usersAndRoles") : null}
                        leftIcon={<GroupIcon sx={{color: adminTheme.palette.brandColor}} />}
                    />
                    <StyledItem
                        to="/settings"
                        primaryText={open ? t("sideMenu.settings") : null}
                        leftIcon={<SettingsIcon sx={{color: adminTheme.palette.brandColor}} />}
                    />
                    <StyledItem
                        to="/sequent_backend_template"
                        primaryText={open && t("sideMenu.communicationTemplates")}
                        leftIcon={<MailIcon sx={{color: adminTheme.palette.brandColor}} />}
                    />
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
