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
import ChecklistIcon from "@mui/icons-material/Checklist"

const StyledItem = styled(Menu.Item)`
    color: ${adminTheme.palette.brandColor};

    &.RaMenuItemLink-active,
    .MuiIconButton-root {
        color: ${adminTheme.palette.brandColor};
    }
`

const StyledMenu = styled(Menu)`
    background-color: ${adminTheme.palette.white};
    color: ${adminTheme.palette.brandColor};
    margin-top: 0;
    margin-right: 4px;
    box-shadow: 0px 2px 1px -1px rgba(0, 0, 0, 0.2), 0px 1px 1px 0px rgba(0, 0, 0, 0.14),
        0px 1px 3px 0px rgba(0, 0, 0, 0.12);
    border-radius: 4px;
`

const DrawerContainer = styled(Box)`
    position: sticky;
    bottom: 0;
    background-color: ${adminTheme.palette.white};
    padding: 8px 16px;
    justify-content: right;
    border-top: 2px solid ${adminTheme.palette.customGrey.light};
    display: flex;
    margin-top: auto;
`

const MenuWrapper = styled(Box)`
    border-bottom: 2px solid ${adminTheme.palette.customGrey.light};
`

export const CustomMenu = () => {
    const [open, setOpen] = useSidebarState()
    const {t} = useTranslation()

    return (
        <StyledMenu
            sx={{
                "flex": "display",
                "flexDirection": "column",
                ".RaMenuItemLink-active": {
                    backgroundColor: adminTheme.palette.green.light,
                },
            }}
        >
            <MenuWrapper>
                <SelectTenants />

                <ElectionEvents />

                <StyledItem
                    to="/user-roles"
                    primaryText={open ? t("sideMenu.usersAndRoles") : null}
                    leftIcon={<GroupIcon sx={{color: adminTheme.palette.brandColor}} />}
                />
                <StyledItem
                    to="/logs"
                    primaryText={open ? t("sideMenu.logs") : null}
                    leftIcon={<ChecklistIcon sx={{color: adminTheme.palette.brandColor}} />}
                />
                <StyledItem
                    to="/settings"
                    primaryText={open ? t("sideMenu.settings") : null}
                    leftIcon={<SettingsIcon sx={{color: adminTheme.palette.brandColor}} />}
                />
                <StyledItem
                    to="/"
                    primaryText={open && t("sideMenu.communicationTemplates")}
                    leftIcon={<MailIcon sx={{color: adminTheme.palette.brandColor}} />}
                />
            </MenuWrapper>

            <DrawerContainer>
                <IconButton
                    icon={open ? faAngleDoubleLeft : faAngleDoubleRight}
                    fontSize="24px"
                    onClick={() => setOpen(!open)}
                />
            </DrawerContainer>
        </StyledMenu>
    )
}
