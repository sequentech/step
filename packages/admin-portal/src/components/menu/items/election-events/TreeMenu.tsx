// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {NavLink} from "react-router-dom"
import React, {useState, useEffect} from "react"
import {ResourceOptions, ResourceDefinition, useResourceDefinitions, useGetList} from "react-admin"
import {CircularProgress} from "@mui/material"
import {useTenantStore} from "../../../CustomMenu"
import {faAngleRight, faAngleDown} from "@fortawesome/free-solid-svg-icons"
import {Icon} from "@sequentech/ui-essentials"
import {cn} from "../../../../lib/utils"

interface Options extends ResourceOptions {
    isMenuParent?: boolean
    menuParent?: string
    foreignKeyFrom?: string
}

interface TreeLeavesProps {
    isOpen: boolean
    resourceName: string
    resourceId?: string
    treeResources: Array<ResourceDefinition<Options>>
    filter?: object
}

function TreeLeaves({isOpen, resourceName, resourceId, treeResources, filter}: TreeLeavesProps) {
    console.log(
        "LS -> src/components/menu/items/election-events/TreeMenu.tsx:28 -> treeResources: ",
        treeResources
    )
    const [tenantId] = useTenantStore()

    const {data, isLoading, error} = useGetList(resourceName, {
        // pagination: {page: 1, perPage: 10},
        sort: {field: "created_at", order: "DESC"},
        filter: resourceId
            ? {
                  tenant_id: tenantId,
                  [treeResources[0]?.options?.foreignKeyFrom || ""]: resourceId,
                  ...filter,
              }
            : {
                  tenant_id: tenantId,
                  ...filter,
              },
    })

    if (isLoading) {
        return <CircularProgress />
    }

    if (error || !data) {
        return null
    }

    const subTreeResources = treeResources.slice(1)
    const nextResourceName = subTreeResources[0]?.name ?? null

    return (
        <div className="bg-white">
            <div className="flex flex-col ml-3">
                {data?.map((resource, idx) => {
                    if (nextResourceName) {
                        return (
                            <TreeMenuItem
                                isOpen={isOpen}
                                resource={resource}
                                resourceName={nextResourceName}
                                treeResources={subTreeResources}
                                key={idx}
                            />
                        )
                    }

                    return <p key={idx}>None</p>
                })}
            </div>
        </div>
    )
}

interface TreeMenuItemProps {
    isOpen: boolean
    resourceName: string
    resource: any
    treeResources: Array<ResourceDefinition<Options>>
}

function TreeMenuItem({isOpen, resourceName, resource, treeResources}: TreeMenuItemProps) {
    const [open, setOpen] = useState(false)
    const onClick = () => setOpen(!open)
    const hasLeaves = treeResources.length > 0

    return (
        <div className="bg-white">
            <div className="flex text-center  space-x-2 items-center">
                {hasLeaves && (
                    <div className="w-6 h-6 cursor-pointer" onClick={onClick}>
                        <Icon icon={open ? faAngleDown : faAngleRight} />
                    </div>
                )}

                {isOpen && (
                    <>
                        <NavLink
                            title={resource.name}
                            className={({isActive}) =>
                                cn(
                                    "px-4 py-1.5 text-secondary border-b-2 border-white hover:border-secondary truncate cursor-pointer",
                                    isActive && "border-b-2 border-brand-color"
                                )
                            }
                            to={`/${resourceName}/${resource.id}`}
                        >
                            {resource.name}
                        </NavLink>
                    </>
                )}
            </div>
            {open && (
                <div className="">
                    <TreeLeaves
                        isOpen={isOpen}
                        resourceName={resourceName}
                        resourceId={resource.id}
                        treeResources={treeResources}
                    />
                </div>
            )}
        </div>
    )
}

export function TreeMenu({isOpen, resourceNames}: {isOpen: boolean; resourceNames: string[]}) {
    const [archivedMenu, setArchivedMenu] = useState(0)

    let allResources = useResourceDefinitions()
    let treeResources = Object.keys(allResources)
        .map((name) => allResources[name])
        .filter((resource) => resourceNames.includes(resource.name))

    function tabChange(val: number) {
        setArchivedMenu(val)
    }

    return (
        <>
            <ul className="flex px-4 space-x-4 bg-white uppercase text-xs leading-6">
                <li
                    className={cn(
                        "px-4 py-1 cursor-pointer",
                        archivedMenu === 0
                            ? "text-brand-color border-b-2 border-brand-success"
                            : "text-secondary"
                    )}
                    onClick={() => tabChange(0)}
                >
                    Active
                </li>
                <li
                    className={cn(
                        "px-4 py-1 cursor-pointer",
                        archivedMenu === 1
                            ? "text-brand-color border-b-2 border-brand-success"
                            : "text-secondary"
                    )}
                    onClick={() => tabChange(1)}
                >
                    Archived
                </li>
            </ul>
            <div className="mx-5 py-2">
                <TreeLeaves
                    resourceName={resourceNames[0]}
                    treeResources={treeResources}
                    isOpen={isOpen}
                    filter={{is_archived: archivedMenu === 1}}
                />
            </div>
        </>
    )
}
