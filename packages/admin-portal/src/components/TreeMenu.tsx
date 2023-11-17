// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box} from "@mui/material"
import React, {useState, useEffect} from "react"
import {
    ResourceOptions,
    ResourceDefinition,
    useResourceDefinitions,
    useGetList,
    MenuItemLink,
} from "react-admin"
import {CircularProgress} from "@mui/material"
import {useTenantStore} from "./CustomMenu"
import {faAngleRight, faAngleDown} from "@fortawesome/free-solid-svg-icons"
import {Icon} from "@sequentech/ui-essentials"
import {styled} from "@mui/material/styles"
import Tabs from "@mui/material/Tabs"
import Tab from "@mui/material/Tab"
import {cn} from "../lib/utils"

interface Options extends ResourceOptions {
    isMenuParent?: boolean
    menuParent?: string
    foreignKeyFrom?: string
}

interface TreeLeavesProps {
    isOpen: boolean
    resourceId?: string
    treeResources: Array<ResourceDefinition<Options>>
    filter?: object
}

const TreeLeaves: React.FC<TreeLeavesProps> = ({isOpen, resourceId, treeResources, filter}) => {
    const [tenantId] = useTenantStore()

    const {data, total, isLoading, error} = useGetList(treeResources[0]?.name || "", {
        pagination: {page: 1, perPage: 10},
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

    return (
        <div className="flex flex-col ml-3">
            {data?.map((resource, idx) => (
                <TreeMenuItem
                    isOpen={isOpen}
                    resourceType={treeResources[0].name}
                    resource={resource}
                    treeResources={treeResources.slice(1)}
                    key={idx}
                />
            ))}
        </div>
    )
}

interface TreeMenuItemProps {
    isOpen: boolean
    resourceType: string
    resource: any
    treeResources: Array<ResourceDefinition<Options>>
}

const TreeMenuItem: React.FC<TreeMenuItemProps> = ({
    isOpen,
    resourceType,
    resource,
    treeResources,
}) => {
    const [open, setOpen] = useState(false)
    const onClick = () => setOpen(!open)
    const hasLeaves = treeResources.length > 0

    return (
        <div className="bg-white">
            <div className="flex text-center cursor-pointer space-x-2 items-center">
                {hasLeaves && (
                    <Icon className="" icon={open ? faAngleDown : faAngleRight} onClick={onClick} />
                )}

                {isOpen && (
                    <MenuItemLink
                        key={resource.name}
                        to={`/${resourceType}/${resource.id}`}
                        primaryText={resource.name}
                    />
                )}
            </div>
            {open && (
                <div className="">
                    <TreeLeaves
                        isOpen={isOpen}
                        resourceId={resource.id}
                        treeResources={treeResources}
                    />
                </div>
            )}
        </div>
    )
}

interface TreeMenuProps {
    isOpen: boolean
}
export const TreeMenu: React.FC<TreeMenuProps> = ({isOpen}) => {
    let allResources = useResourceDefinitions()
    let [treeResources, setTreeResources] = useState<Array<ResourceDefinition<Options>>>([])
    const [archivedMenu, setArchivedMenu] = useState(0)

    useEffect(() => {
        const resources: Array<ResourceDefinition<Options>> = Object.keys(allResources).map(
            (name) => allResources[name]
        )
        let parent = resources.find((resource) => resource.options?.isMenuParent)

        if (!parent) {
            return
        }

        let tree: Array<ResourceDefinition<Options>> = [parent]
        let finished = false
        while (!finished) {
            let lastLeave = tree[tree.length - 1]
            let leave = resources.find(
                (resource) => resource.options?.menuParent === lastLeave.name
            )
            if (!leave) {
                finished = true
            } else {
                tree.push(leave)
            }
        }
        setTreeResources(tree)
    }, [allResources])

    const handleChange = (event: React.SyntheticEvent, newValue: number) => {
        console.log(`new value ${newValue}`)
        setArchivedMenu(newValue)
    }

    function tabChange(val: number) {
        console.log(`new value ${val}`)
        setArchivedMenu(val)
    }

    if (0 === treeResources.length) {
        return null
    }

    return (
        <>
            <ul className="flex space-x-4 bg-white text-secondary uppercase text-xs leading-6">
                <li className="px-4 py-1 cursor-pointer" onClick={() => tabChange(0)}>
                    Active
                </li>
                <li className="px-4 py-1 cursor-pointer" onClick={() => tabChange(1)}>
                    Archived
                </li>
            </ul>
            <div className="mx-5 my-2">
                <TreeLeaves
                    treeResources={treeResources}
                    isOpen={isOpen}
                    filter={{is_archived: 1 === archivedMenu}}
                />
            </div>
        </>
    )
}
