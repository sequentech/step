// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {NavLink} from "react-router-dom"
import React, {useRef, useState} from "react"
import {ResourceOptions, ResourceDefinition, useResourceDefinitions, useGetList} from "react-admin"
import {
    CircularProgress,
    Divider,
    ListItemIcon,
    ListItemText,
    MenuItem,
    MenuList,
    Popover,
} from "@mui/material"
import {useTenantStore} from "../../../CustomMenu"
import {
    faAngleRight,
    faAngleDown,
    faEllipsisH,
    faCirclePlus,
    faTrash,
    faArchive,
} from "@fortawesome/free-solid-svg-icons"
import {adminTheme, Icon} from "@sequentech/ui-essentials"
import {cn} from "../../../../lib/utils"
import styled from "@emotion/styled"

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

function TreeLeaves({isOpen, resourceName, treeResources, filter}: TreeLeavesProps) {
    const [tenantId] = useTenantStore()

    const {data, isLoading, error} = useGetList(resourceName, {
        // pagination: {page: 1, perPage: 10},
        sort: {field: "created_at", order: "DESC"},
        filter: {
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
        <div className="bg-white">
            <div className="flex flex-col ml-3">
                {data?.map((resource, idx) => {
                    return (
                        <TreeMenuItem
                            isOpen={isOpen}
                            resource={resource}
                            treeResources={treeResources}
                            key={idx}
                        />
                    )
                })}
            </div>
        </div>
    )
}

interface TreeMenuItemProps {
    isOpen: boolean
    resource: any
    treeResources: Array<ResourceDefinition<Options>>
}

enum Action {
    Add,
    Remove,
    Archive,
}

type ActionPayload = {
    id: string
    name: string
    type: string
}

function TreeMenuItem({isOpen, resource, treeResources}: TreeMenuItemProps) {
    const [open, setOpen] = useState(false)
    const onClick = () => setOpen(!open)

    const subTreeResources = treeResources.slice(1)
    const nextResource = subTreeResources[0] ?? null
    const hasNext = !!nextResource

    const menuItemRef = useRef(null)
    const [anchorEl, setAnchorEl] = React.useState<HTMLParagraphElement | null>(null)

    function handleOpenItemActions(event: React.MouseEvent<HTMLParagraphElement>): void {
        setAnchorEl(menuItemRef.current)
    }

    function handleAction(action: Action, payload: ActionPayload) {
        console.log(
            "LS -> src/components/menu/items/election-events/TreeMenu.tsx:115 -> payload: ",
            payload
        )
    }

    const handleCloseActionMenu = () => {
        setAnchorEl(null)
    }

    const openActionMenu = Boolean(anchorEl)
    const idActionMenu = openActionMenu ? "action-menu" : undefined

    const StyledIcon = styled(Icon)`
        color: ${adminTheme.palette.brandColor};
    `

    return (
        <div className="bg-white">
            <div ref={menuItemRef} className="group flex text-center space-x-2 items-center">
                {hasNext ? (
                    <div className="w-6 h-6 cursor-pointer" onClick={onClick}>
                        <Icon icon={open ? faAngleDown : faAngleRight} />
                    </div>
                ) : (
                    <div className="w-6 h-6"></div>
                )}
                {isOpen && (
                    <NavLink
                        title={resource.alias ?? resource.name}
                        className={({isActive}) =>
                            cn(
                                "px-4 py-1.5 text-secondary border-b-2 border-white hover:border-secondary truncate cursor-pointer",
                                isActive && "border-b-2 border-brand-color"
                            )
                        }
                        to={`/${treeResources[0].name}/${resource.id}`}
                    >
                        {resource.name}
                    </NavLink>
                )}
                <div className="grow hidden group-hover:block">
                    <p className="text-right px-1 cursor-pointer" onClick={handleOpenItemActions}>
                        <Icon icon={faEllipsisH} />
                    </p>
                    <Popover
                        id={idActionMenu}
                        open={openActionMenu}
                        anchorEl={anchorEl}
                        onClose={handleCloseActionMenu}
                        anchorOrigin={{
                            vertical: "bottom",
                            horizontal: "right",
                        }}
                    >
                        <MenuList dense>
                            <MenuItem
                                onClick={() =>
                                    handleAction(Action.Add, {
                                        id: resource.id,
                                        name: resource.name,
                                        type: treeResources[0].name,
                                    })
                                }
                            >
                                <ListItemIcon>
                                    <StyledIcon icon={faCirclePlus} />
                                </ListItemIcon>
                                Add
                            </MenuItem>
                            <Divider />
                            <MenuItem
                                onClick={() =>
                                    handleAction(Action.Remove, {
                                        id: resource.id,
                                        name: resource.name,
                                        type: treeResources[0].name,
                                    })
                                }
                            >
                                <ListItemIcon>
                                    <StyledIcon icon={faTrash} />
                                </ListItemIcon>
                                Remove
                            </MenuItem>
                            <Divider />
                            <MenuItem
                                onClick={() =>
                                    handleAction(Action.Archive, {
                                        id: resource.id,
                                        name: resource.name,
                                        type: treeResources[0].name,
                                    })
                                }
                            >
                                <ListItemIcon>
                                    <StyledIcon icon={faArchive} />
                                </ListItemIcon>
                                Archive
                            </MenuItem>
                        </MenuList>
                    </Popover>
                </div>
            </div>
            {open && (
                <div className="">
                    {hasNext && (
                        <TreeLeaves
                            isOpen={isOpen}
                            resourceName={nextResource.name}
                            treeResources={subTreeResources}
                        />
                    )}
                </div>
            )}
        </div>
    )
}

export function TreeMenu({
    isOpen,
    resourceNames,
    filter,
}: {
    isOpen: boolean
    resourceNames: string[]
    filter: object
}) {
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
                    filter={Object.assign({}, {is_archived: archivedMenu === 1}, filter)}
                />
            </div>
        </>
    )
}
