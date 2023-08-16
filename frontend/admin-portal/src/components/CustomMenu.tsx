// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Menu, useSidebarState, useGetList} from "react-admin"
import {faThLarge, faUsers, faCog, faStar, faPlusCircle} from "@fortawesome/free-solid-svg-icons"
import {IconButton} from "@sequentech/ui-essentials"
import {HorizontalBox} from "./HorizontalBox"
import {Box, MenuItem, Select, SelectChangeEvent} from "@mui/material"
import {useStore} from "ra-core"

export const useTenantStore = () => useStore<string | null>("tenant_id", null)

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
                    <IconButton icon={faPlusCircle} fontSize="24px" />
                </Box>
            ) : null}
        </HorizontalBox>
    )
}

export const CustomMenu = () => (
    <>
        <Menu>
            <CustomerSelector />
            <Menu.Item
                to="/sequent_backend_election_event"
                primaryText="Election Events"
                leftIcon={<IconButton icon={faThLarge} fontSize="24px" />}
            />
            <Menu.Item
                to="/user-roles"
                primaryText="User and Roles"
                leftIcon={<IconButton icon={faUsers} fontSize="24px" />}
            />
            <Menu.Item
                to="/settings"
                primaryText="Settings"
                leftIcon={<IconButton icon={faCog} fontSize="24px" />}
            />
            <Menu.Item
                to="/messages"
                primaryText="Messages"
                leftIcon={<IconButton icon={faStar} fontSize="24px" />}
            />
        </Menu>
    </>
)
