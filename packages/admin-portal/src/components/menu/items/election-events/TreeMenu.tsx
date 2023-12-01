// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useRef, useState} from "react"
import {NavLink} from "react-router-dom"
import {useSidebarState} from "react-admin"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"
import ChevronRightIcon from "@mui/icons-material/ChevronRight"
import HowToVoteIcon from "@mui/icons-material/HowToVote"
import AddIcon from "@mui/icons-material/Add"
import {cn} from "@/lib/utils"

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

import MenuActions from "./MenuActions"
import {useActionPermissions} from "../use-tree-menu-hook"

export const mapAddResource: Record<ResourceName, string> = {
    sequent_backend_election_event: "sideMenu.addResource.electionEvent",
    sequent_backend_election: "sideMenu.addResource.election",
    sequent_backend_contest: "sideMenu.addResource.contest",
    sequent_backend_candidate: "sideMenu.addResource.candidate",
}

export function getNavLinkCreate(
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

    const {canCreateElectionEvent} = useActionPermissions()

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
                {!isArchivedElectionEvents && canCreateElectionEvent && (
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

function TreeMenuItem({
    resource,
    parentData,
    id,
    name,
    treeResourceNames,
    isArchivedElectionEvents,
}: TreeMenuItemProps) {
    const {t} = useTranslation()
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

    const menuItemRef = useRef<HTMLDivElement | null>(null)

    return (
        <div className="bg-white">
            <div ref={menuItemRef} className="group flex text-left space-x-2 items-center">
                {hasNext ? (
                    <div
                        className="flex-none w-6 h-6 cursor-pointer text-customGrey-dark"
                        onClick={onClick}
                    >
                        {open ? <ExpandMoreIcon /> : <ChevronRightIcon />}
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
                    <MenuActions
                        isArchivedTab={isArchivedElectionEvents}
                        resourceId={id}
                        resourceName={name}
                        resourceType={treeResourceNames[0]}
                        parentData={parentData}
                        menuItemRef={menuItemRef}
                    ></MenuActions>
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
