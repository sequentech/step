// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect} from "react"
import {useSidebarState, useGetList, useRefresh} from "react-admin"
import {faThLarge, faPlusCircle} from "@fortawesome/free-solid-svg-icons"
import {IconButton} from "@sequentech/ui-essentials"
import {MenuItem, Select, SelectChangeEvent} from "@mui/material"
import {Link} from "react-router-dom"
import {cn} from "../../../lib/utils"
import {AuthContext} from "../../../providers/AuthContextProvider"
import {useTenantStore} from "../../../providers/TenantContextProvider"

const SelectTenants: React.FC = () => {
    const [open] = useSidebarState()
    const [tenantId, setTenantId] = useTenantStore()
    const refresh = useRefresh()
    const authContext = useContext(AuthContext)

    useEffect(() => {
        console.log(`FF 2 ${tenantId}`)
    }, [tenantId])

    const {data, total, isLoading, error} = useGetList("sequent_backend_tenant", {
        pagination: {page: 1, perPage: 10},
        sort: {field: "updated_at", order: "DESC"},
        filter: {is_active: true},
    })

    const showCustomers = open && !isLoading && !error && !!data

    useEffect(() => {
        if (!tenantId && authContext.tenantId) {
            setTenantId(authContext.tenantId)
        }
    }, [tenantId, authContext.tenantId])

    useEffect(() => {
        console.log(`${data}, ${open}, ${isLoading}, ${error} ${showCustomers}`)
    }, [data, total, isLoading, error, showCustomers])

    const hasSingle = total === 1

    const handleChange = (event: SelectChangeEvent<unknown>) => {
        const tenantId: string = event.target.value as string
        setTenantId(tenantId)
        refresh()
    }

    return (
        <div className={cn("flex items-center px-4 space-x-4", hasSingle ? "py-1.5" : "py-1")}>
            <IconButton icon={faThLarge} />
            {!!data && (
                <>
                    {hasSingle ? (
                        <p className="ml-2.5">{data[0].slug}</p>
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
                                    {tenant.slug}
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
