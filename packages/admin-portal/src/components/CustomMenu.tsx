// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Menu, useSidebarState} from "react-admin"
import {
    faUsers,
    faCog,
    faAngleDoubleLeft,
    faAngleDoubleRight,
    faEnvelope,
} from "@fortawesome/free-solid-svg-icons"
import {IconButton, adminTheme} from "@sequentech/ui-essentials"
import {Box} from "@mui/material"
import {styled} from "@mui/material/styles"
import SelectTenants from "./menu/items/SelectTenants"
import ElectionEvents from "./menu/items/ElectionEvents"

export const useTenantStore: () => [string | null, (tenantId: string | null) => void] = () => {
    return [
        localStorage.getItem("selected-tenant-id"),
        (tenantId: string | null) => localStorage.setItem("selected-tenant-id", tenantId || ""),
    ]
}

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

                {
                    // <StyledItem
                    //     to="/pgaudit"
                    //     primaryText={open ? "PG Audit" : null}
                    //     leftIcon={<IconButton icon={faThLarge} fontSize="24px" />}
                    // />
                    // <StyledItem
                    //     to="/sequent_backend_area"
                    //     primaryText={open ? "Areas" : null}
                    //     leftIcon={<IconButton icon={faThLarge} fontSize="24px" />}
                    // />
                    // <StyledItem
                    //     to="/sequent_backend_area_contest"
                    //     primaryText={open ? "Area Contests" : null}
                    //     leftIcon={<IconButton icon={faThLarge} fontSize="24px" />}
                    // />
                    // <StyledItem
                    //     to="/sequent_backend_ballot_style"
                    //     primaryText={open ? "Ballot Styles" : null}
                    //     leftIcon={<IconButton icon={faThLarge} fontSize="24px" />}
                    // />
                    // <StyledItem
                    //     to="/sequent_backend_tenant"
                    //     primaryText={open ? "Customers" : null}
                    //     leftIcon={<IconButton icon={faThLarge} fontSize="24px" />}
                    // />
                    // <StyledItem
                    //     to="/sequent_backend_document"
                    //     primaryText={open ? "Documents" : null}
                    //     leftIcon={<IconButton icon={faFileText} fontSize="24px" />}
                    // />
                    // <StyledItem
                    //     to="/sequent_backend_trustee"
                    //     primaryText={open ? "Trustees" : null}
                    //     leftIcon={<IconButton icon={faFileText} fontSize="24px" />}
                    // />
                    // <StyledItem
                    //     to="/messages"
                    //     primaryText={open ? "Messages" : null}
                    //     leftIcon={<IconButton icon={faStar} fontSize="24px" />}
                    // />
                }

                <StyledItem
                    to="/user-roles"
                    primaryText={open ? "User and Roles" : null}
                    leftIcon={<IconButton icon={faUsers} fontSize="24px" />}
                />
                <StyledItem
                    to="/settings"
                    primaryText={open ? "Settings" : null}
                    leftIcon={<IconButton icon={faCog} fontSize="24px" />}
                />
                <StyledItem
                    to="/"
                    primaryText={open && "Communication Templates"}
                    leftIcon={<IconButton icon={faEnvelope} fontSize="24px" />}
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
