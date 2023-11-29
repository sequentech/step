// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {NavLink, useNavigate} from "react-router-dom"
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
import ExpandLessIcon from "@mui/icons-material/ExpandLess"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"
import HowToVoteIcon from "@mui/icons-material/HowToVote"
import AddIcon from "@mui/icons-material/Add"
import {adminTheme, Icon} from "@sequentech/ui-essentials"
import {cn} from "../../../../lib/utils"
import styled from "@emotion/styled"
import {
    mapDataChildren,
    ResourceName,
    DataTreeMenuType,
    DynEntityType,
    ElectionType,
    ContestType,
    CandidateType,
} from "../ElectionEvents"
import {useTranslation} from "react-i18next"

const mapAddResource: Record<ResourceName, string> = {
    sequent_backend_election_event: "sideMenu.addResource.addElectionEvent",
    sequent_backend_election: "sideMenu.addResource.addElection",
    sequent_backend_contest: "sideMenu.addResource.addContest",
    sequent_backend_candidate: "sideMenu.addResource.addCandidate",
}

function getNavLinkCreate(
    resource: DataTreeMenuType | undefined,
    resourceName: ResourceName
): string {
    const params: Record<string, string> = {}

    switch (resourceName) {
        case "sequent_backend_election":
            params.electionEventId = (resource as ElectionType).id
            break
        case "sequent_backend_contest":
            params.electionEventId = (resource as ContestType).election_event_id
            params.electionId = (resource as ContestType).id
            break
        case "sequent_backend_candidate":
            params.electionEventId = (resource as CandidateType).election_event_id
            params.contestId = (resource as CandidateType).id
            break
    }

    const url = `/${resourceName}/create`

    const urlObject = new URL(url, window.location.origin)

    Object.entries(params).forEach(([key, value]) => {
        urlObject.searchParams.append(key, value.toString())
    })

    const res = urlObject.pathname + urlObject.search

    return res
}

interface TreeLeavesProps {
    data: DynEntityType
    parentData: DataTreeMenuType
    treeResourceNames: ResourceName[]
    isArchivedElectionEvents: boolean
}

function TreeLeaves({
    data,
    parentData,
    treeResourceNames,
    isArchivedElectionEvents,
}: TreeLeavesProps) {
    const {t} = useTranslation()

    return (
        <div className="bg-white">
            <div className="flex flex-col ml-3">
                {data?.[mapDataChildren(treeResourceNames[0])]?.map(
                    (resource: DataTreeMenuType) => {
                        return (
                            <TreeMenuItem
                                key={resource.id}
                                resource={resource}
                                parentData={resource}
                                id={resource.id}
                                name={resource.name}
                                treeResourceNames={treeResourceNames}
                                isArchivedElectionEvents={isArchivedElectionEvents}
                            />
                        )
                    }
                )}
                {!isArchivedElectionEvents && (
                    <div className="flex items-center space-x-2 text-secondary">
                        <AddIcon className="flex-none"></AddIcon>
                        <NavLink
                            className="grow py-1.5 border-b-2 border-white hover:border-secondary truncate cursor-pointer"
                            to={getNavLinkCreate(parentData, treeResourceNames[0])}
                        >
                            {t(mapAddResource[treeResourceNames[0] as ResourceName])}
                        </NavLink>
                        <div className="flex-none w-6 h-6 invisible"></div>
                    </div>
                )}
            </div>
        </div>
    )
}

interface TreeMenuItemProps {
    resource: DataTreeMenuType
    parentData: DataTreeMenuType
    id: string
    name: string
    treeResourceNames: ResourceName[]
    isArchivedElectionEvents: boolean
}

enum Action {
    Add,
    Remove,
    Archive,
}

type ActionPayload = {
    id: string
    name: string
    type: ResourceName
}

function TreeMenuItem({
    resource,
    parentData,
    id,
    name,
    treeResourceNames,
    isArchivedElectionEvents,
}: TreeMenuItemProps) {
    const {t} = useTranslation()
    const navigate = useNavigate()
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

    function handleAction(action: Action, payload: ActionPayload) {
        // close the popover
        setAnchorEl(null)

        if (action === Action.Add) {
            navigate(getNavLinkCreate(parentData, payload.type))
        }
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
            <div ref={menuItemRef} className="group flex text-left space-x-2 items-center">
                {hasNext ? (
                    <div
                        className="flex-none w-6 h-6 cursor-pointer text-customGrey-dark"
                        onClick={onClick}
                    >
                        {open ? <ExpandLessIcon /> : <ExpandMoreIcon />}
                    </div>
                ) : (
                    <div className="flex-none w-6 h-6"></div>
                )}
                {isOpenSidebar && (
                    <NavLink
                        title={name}
                        className={({isActive}) =>
                            cn(
                                "grow py-1.5 text-black border-b-2 border-white hover:border-brand-color truncate cursor-pointer",
                                isActive && "border-b-2 border-brand-color"
                            )
                        }
                        to={`/${treeResourceNames[0]}/${id}`}
                    >
                        {treeResourceNames[0] === "sequent_backend_election_event" ? (
                            <p className="flex items-center space-x-2">
                                <HowToVoteIcon className="text-brand-color" />
                                <span>{name}</span>
                            </p>
                        ) : (
                            <span>{name}</span>
                        )}
                    </NavLink>
                )}
                <div className="invisible group-hover:visible">
                    <p
                        className="flex-none w-6 h-6 text-right px-1 cursor-pointer"
                        onClick={handleOpenItemActions}
                    >
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
                                {t(mapAddResource[treeResourceNames[0] as ResourceName])}
                            </MenuItem>
                            {
                                // <Divider />
                                // <MenuItem
                                //     onClick={() =>
                                //         handleAction(Action.Remove, {
                                //             id,
                                //             name,
                                //             type: treeResourceNames[0],
                                //         })
                                //     }
                                // >
                                //     <ListItemIcon>
                                //         <StyledIcon icon={faTrash} />
                                //     </ListItemIcon>
                                //     Remove
                                // </MenuItem>
                                // <Divider />
                                // <MenuItem
                                //     onClick={() =>
                                //         handleAction(Action.Archive, {
                                //             id,
                                //             name,
                                //             type: treeResourceNames[0],
                                //         })
                                //     }
                                // >
                                //     <ListItemIcon>
                                //         <StyledIcon icon={faArchive} />
                                //     </ListItemIcon>
                                //     Archive
                                // </MenuItem>
                            }
                        </MenuList>
                    </Popover>
                </div>
            </div>
            {open && (
                <div className="">
                    {hasNext && (
                        <TreeLeaves
                            data={data}
                            parentData={parentData}
                            treeResourceNames={subTreeResourceNames}
                            isArchivedElectionEvents={isArchivedElectionEvents}
                        />
                    )}
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
    treeResourceNames: ResourceName[]
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
                <TreeLeaves
                    data={data}
                    parentData={data as DataTreeMenuType}
                    treeResourceNames={treeResourceNames}
                    isArchivedElectionEvents={isArchivedElectionEvents}
                />
            </div>
        </>
    )
}
