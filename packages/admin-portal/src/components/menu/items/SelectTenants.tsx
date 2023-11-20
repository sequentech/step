// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {useSidebarState, useGetList} from "react-admin"
import {faThLarge, faPlusCircle} from "@fortawesome/free-solid-svg-icons"
import {IconButton} from "@sequentech/ui-essentials"
import {MenuItem, Select, SelectChangeEvent} from "@mui/material"
import {Link} from "react-router-dom"
import {useTenantStore} from "../../CustomMenu"
import {cn} from "../../../lib/utils"

const SelectTenants: React.FC = () => {
    const [open] = useSidebarState()
    const [tenantId, setTenantId] = useTenantStore()

    const {data, total, isLoading, error} = useGetList("sequent_backend_tenant", {
        pagination: {page: 1, perPage: 10},
        sort: {field: "updated_at", order: "DESC"},
        filter: {is_active: true},
    })

    const showCustomers = open && !isLoading && !error && !!data

    const hasSingle = total === 1

    const handleChange = (event: SelectChangeEvent<unknown>) => {
        const tenantId: string = event.target.value as string
        setTenantId(tenantId)
    }

    return (
        <div className={cn("flex items-center px-4 space-x-4", hasSingle ? "py-1.5" : "py-1")}>
            <IconButton icon={faThLarge} />
            {showCustomers && (
                <>
                    {hasSingle ? (
                        <p className="ml-2.5">{data[0].username}</p>
                    ) : (
                        <Select
                            labelId="tenant-select-label"
                            id="tenant-select"
                            value={tenantId}
                            onChange={handleChange}
                            className="grow mx-0 !-my-0"
                        >
                            {data?.map((tenant) => (
                                <MenuItem key={tenant.id} value={tenant.id}>
                                    {tenant.username}
                                </MenuItem>
                            ))}
                        </Select>
                    )}
                    <Link to="/sequent_backend_tenant/create">
                        <IconButton className="text-brand-color text-base" icon={faPlusCircle} />
                    </Link>
                </>
            )}
        </div>
    )
}

export default SelectTenants
