// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect} from "react"
import {Menu, useSidebarState, useGetList, useResourceContext} from "react-admin"
import {
    faThLarge,
    faUsers,
    faCog,
    faStar,
    faPlusCircle,
    faFileText,
} from "@fortawesome/free-solid-svg-icons"
import {IconButton, adminTheme} from "@sequentech/ui-essentials"
import {HorizontalBox} from "./HorizontalBox"
import {Box, MenuItem, Paper, Select, SelectChangeEvent} from "@mui/material"
import {styled} from "@mui/material/styles"
import {Link} from "react-router-dom"
import {TreeMenu} from "./TreeMenu"

export const useTenantStore: () => [string | null, (tenantId: string | null) => void] = () => {
    return [
        localStorage.getItem("tenantId"),
        (tenantId: string | null) => localStorage.setItem("tenantId", tenantId || ""),
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

const CustomerSelector: React.FC = () => {
    const [open] = useSidebarState()
    const [tenant, setTenant] = useTenantStore()

    const {data, total, isLoading, error} = useGetList("sequent_backend_tenant", {
        pagination: {page: 1, perPage: 10},
        sort: {field: "updated_at", order: "DESC"},
        filter: {is_active: true},
    })

    const showCustomers = open && !isLoading && !error

    const handleChange = (event: SelectChangeEvent<string | null>) => {
        setTenant(event.target.value)
    }

    return (
        <HorizontalBox>
            <IconButton icon={faThLarge} fontSize="24px" />
            {showCustomers ? (
                <Box>
                    <Select
                        labelId="tenant-select-label"
                        id="tenant-select"
                        value={tenant}
                        onChange={handleChange}
                    >
                        {data?.map((tenant) => (
                            <MenuItem key={tenant.id} value={tenant.id}>
                                {tenant.username}
                            </MenuItem>
                        ))}
                    </Select>
                    <Link to="/sequent_backend_tenant/create">
                        <IconButton icon={faPlusCircle} fontSize="24px" />
                    </Link>
                </Box>
            ) : null}
        </HorizontalBox>
    )
}

export const CustomMenu = () => {
    const [open] = useSidebarState()
    const resource = useResourceContext()

    useEffect(() => {
        console.log(resource)
    }, [resource])

    return (
        <StyledMenu
            sx={{
                ".RaMenuItemLink-active": {
                    backgroundColor: adminTheme.palette.green.light,
                },
                "&.RaMenu-open": {
                    width: "296px",
                },
            }}
        >
            <CustomerSelector />
            <TreeMenu isOpen={open} />
            <StyledItem
                to="/pgaudit"
                primaryText={open ? "PG Audit" : null}
                leftIcon={<IconButton icon={faThLarge} fontSize="24px" />}
            />
            <StyledItem
                to="/sequent_backend_area"
                primaryText={open ? "Areas" : null}
                leftIcon={<IconButton icon={faThLarge} fontSize="24px" />}
            />
            <StyledItem
                to="/sequent_backend_area_contest"
                primaryText={open ? "Area Contests" : null}
                leftIcon={<IconButton icon={faThLarge} fontSize="24px" />}
            />
            <StyledItem
                to="/sequent_backend_ballot_style"
                primaryText={open ? "Ballot Styles" : null}
                leftIcon={<IconButton icon={faThLarge} fontSize="24px" />}
            />
            <StyledItem
                to="/sequent_backend_tenant"
                primaryText={open ? "Customers" : null}
                leftIcon={<IconButton icon={faThLarge} fontSize="24px" />}
            />
            <StyledItem
                to="/sequent_backend_document"
                primaryText={open ? "Documents" : null}
                leftIcon={<IconButton icon={faFileText} fontSize="24px" />}
            />
            <StyledItem
                to="/sequent_backend_trustee"
                primaryText={open ? "Trustees" : null}
                leftIcon={<IconButton icon={faFileText} fontSize="24px" />}
            />
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
                to="/messages"
                primaryText={open ? "Messages" : null}
                leftIcon={<IconButton icon={faStar} fontSize="24px" />}
            />
        </StyledMenu>
    )
}
