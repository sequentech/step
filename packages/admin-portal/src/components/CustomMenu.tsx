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
import {IconButton, theme} from "@sequentech/ui-essentials"
import {HorizontalBox} from "./HorizontalBox"
import {Box, MenuItem, Select, SelectChangeEvent} from "@mui/material"
import {styled} from "@mui/material/styles"
import {Link} from "react-router-dom"
import { TreeMenu } from "./TreeMenu"

export const useTenantStore: () => [string | null, (tenantId: string | null) => void] = () => {
    return [
        localStorage.getItem("tenantId"),
        (tenantId: string | null) => localStorage.setItem("tenantId", tenantId || ""),
    ]
}

const StyledItem = styled(Menu.Item)`
    color: ${theme.palette.brandColor};

    &.RaMenuItemLink-active,
    .MuiIconButton-root {
        color: ${theme.palette.brandColor};
    }
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
    const resource = useResourceContext()

    useEffect(() => {
        console.log(resource)
    }, [resource])

    return (
        <Menu
            sx={{
                ".RaMenuItemLink-active": {
                    backgroundColor: theme.palette.green.light,
                },
                "color": theme.palette.brandColor,
            }}
        >
            <CustomerSelector />
            <TreeMenu />
            <StyledItem
                to="/pgaudit"
                primaryText="PG Audit"
                leftIcon={<IconButton icon={faThLarge} fontSize="24px" />}
            />
            <StyledItem
                to="/sequent_backend_election_event"
                primaryText="Election Events"
                leftIcon={<IconButton icon={faThLarge} fontSize="24px" />}
            />
            <StyledItem
                to="/sequent_backend_election"
                primaryText="Elections"
                leftIcon={<IconButton icon={faThLarge} fontSize="24px" />}
            />
            <StyledItem
                to="/sequent_backend_contest"
                primaryText="Contests"
                leftIcon={<IconButton icon={faThLarge} fontSize="24px" />}
            />
            <StyledItem
                to="/sequent_backend_candidate"
                primaryText="Candidates"
                leftIcon={<IconButton icon={faThLarge} fontSize="24px" />}
            />
            <StyledItem
                to="/sequent_backend_area"
                primaryText="Areas"
                leftIcon={<IconButton icon={faThLarge} fontSize="24px" />}
            />
            <StyledItem
                to="/sequent_backend_area_contest"
                primaryText="Area Contests"
                leftIcon={<IconButton icon={faThLarge} fontSize="24px" />}
            />
            <StyledItem
                to="/sequent_backend_ballot_style"
                primaryText="Ballot Styles"
                leftIcon={<IconButton icon={faThLarge} fontSize="24px" />}
            />
            <StyledItem
                to="/sequent_backend_tenant"
                primaryText="Customers"
                leftIcon={<IconButton icon={faThLarge} fontSize="24px" />}
            />
            <StyledItem
                to="/sequent_backend_document"
                primaryText="Documents"
                leftIcon={<IconButton icon={faFileText} fontSize="24px" />}
            />
            <StyledItem
                to="/sequent_backend_trustee"
                primaryText="Trustees"
                leftIcon={<IconButton icon={faFileText} fontSize="24px" />}
            />
            <StyledItem
                to="/user-roles"
                primaryText="User and Roles"
                leftIcon={<IconButton icon={faUsers} fontSize="24px" />}
            />
            <StyledItem
                to="/settings"
                primaryText="Settings"
                leftIcon={<IconButton icon={faCog} fontSize="24px" />}
            />
            <StyledItem
                to="/messages"
                primaryText="Messages"
                leftIcon={<IconButton icon={faStar} fontSize="24px" />}
            />
        </Menu>
    )
}
