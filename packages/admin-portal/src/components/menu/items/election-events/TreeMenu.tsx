// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {NavLink} from "react-router-dom"
import React, {useRef, useState} from "react"
import {useSidebarState} from "react-admin"
import {Divider, ListItemIcon, MenuItem, MenuList, Popover} from "@mui/material"
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
import {mapDataChildren, ResourceName, DataTreeMenuType, DynEntityType} from "../ElectionEvents"
import {useTranslation} from "react-i18next"

interface TreeLeavesProps {
    data: DynEntityType
    treeResourceNames: string[]
}

function TreeLeaves({data, treeResourceNames}: TreeLeavesProps) {
    return (
        <div className="bg-white">
            <div className="flex flex-col ml-3">
                {data?.[mapDataChildren(treeResourceNames[0] as ResourceName)]?.map(
                    (resource: DataTreeMenuType) => {
                        return (
                            <TreeMenuItem
                                key={resource.id}
                                resource={resource}
                                id={resource.id}
                                name={resource.name}
                                treeResourceNames={treeResourceNames}
                            />
                        )
                    }
                )}
            </div>
        </div>
    )
}

interface TreeMenuItemProps {
    resource: DataTreeMenuType
    id: string
    name: string
    treeResourceNames: string[]
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

function TreeMenuItem({resource, id, name, treeResourceNames}: TreeMenuItemProps) {
    const [isOpenSidebar] = useSidebarState()

    const [open, setOpen] = useState(false)
    const onClick = () => setOpen(!open)

    const subTreeResourceNames = treeResourceNames.slice(1)
    const nextResourceName = subTreeResourceNames[0] ?? null
    const hasNext = !!nextResourceName

    let data: DynEntityType = {}
    if (hasNext) {
        const key = mapDataChildren(subTreeResourceNames[0] as ResourceName)
        data[key] = (resource as any)[key]
    }

    const menuItemRef = useRef(null)
    const [anchorEl, setAnchorEl] = React.useState<HTMLParagraphElement | null>(null)

    function handleOpenItemActions(): void {
        setAnchorEl(menuItemRef.current)
    }

    function handleAction(_action: Action, _payload: ActionPayload) {
        // TODO
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
                {isOpenSidebar && (
                    <NavLink
                        title={name}
                        className={({isActive}) =>
                            cn(
                                "px-4 py-1.5 text-secondary border-b-2 border-white hover:border-secondary truncate cursor-pointer",
                                isActive && "border-b-2 border-brand-color"
                            )
                        }
                        to={`/${treeResourceNames[0]}/${id}`}
                    >
                        {name}
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
                                        id,
                                        name,
                                        type: treeResourceNames[0],
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
                                        id,
                                        name,
                                        type: treeResourceNames[0],
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
                                        id,
                                        name,
                                        type: treeResourceNames[0],
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
                    {hasNext && <TreeLeaves data={data} treeResourceNames={subTreeResourceNames} />}
                </div>
            )}
        </div>
    )
}

export function TreeMenu({
    data,
    treeResourceNames,
    isArchivedElectionEvents,
    onArchiveElectionEventsSelect,
}: {
    data: DynEntityType
    treeResourceNames: string[]
    isArchivedElectionEvents: boolean
    onArchiveElectionEventsSelect: (val: number) => void
}) {
    const {t} = useTranslation()
    return (
        <>
            <ul className="flex px-4 space-x-4 bg-white uppercase text-xs leading-6">
                <li
                    className={cn(
                        "px-4 py-1 cursor-pointer",
                        !isArchivedElectionEvents
                            ? "text-brand-color border-b-2 border-brand-success"
                            : "text-secondary"
                    )}
                    onClick={() => onArchiveElectionEventsSelect(0)}
                >
                    {t("sideMenu.active")}
                </li>
                <li
                    className={cn(
                        "px-4 py-1 cursor-pointer",
                        isArchivedElectionEvents
                            ? "text-brand-color border-b-2 border-brand-success"
                            : "text-secondary"
                    )}
                    onClick={() => onArchiveElectionEventsSelect(1)}
                >
                    {t("sideMenu.archived")}
                </li>
            </ul>
            <div className="mx-5 py-2">
                <TreeLeaves data={data} treeResourceNames={treeResourceNames} />
            </div>
        </>
    )
}
