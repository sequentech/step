// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useMemo, useRef, useState} from "react"
import {NavLink} from "react-router-dom"
import {useGetOne, useSidebarState} from "react-admin"
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
import {useTenantStore} from "@/providers/TenantContextProvider"
import {NewResourceContext} from "@/providers/NewResourceProvider"

export const mapAddResource: Record<ResourceName, string> = {
    sequent_backend_election_event: "createResource.electionEvent",
    sequent_backend_election: "createResource.election",
    sequent_backend_contest: "createResource.contest",
    sequent_backend_candidate: "createResource.candidate",
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
                                superParentData={parentData}
                                id={resource.id}
                                name={resource.name}
                                treeResourceNames={treeResourceNames}
                                isArchivedElectionEvents={isArchivedElectionEvents}
                                canCreateElectionEvent={canCreateElectionEvent}
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
    superParentData: DataTreeMenuType
    id: string
    name: string
    treeResourceNames: ResourceName[]
    isArchivedElectionEvents: boolean
    canCreateElectionEvent: boolean
}

function TreeMenuItem({
    resource,
    parentData,
    superParentData,
    id,
    name,
    treeResourceNames,
    isArchivedElectionEvents,
    canCreateElectionEvent,
}: TreeMenuItemProps) {
    const [isOpenSidebar] = useSidebarState()

    const [open, setOpen] = useState(false)
    // const [isFirstLoad, setIsFirstLoad] = useState(true)

    const onClick = () => setOpen(!open)

    const subTreeResourceNames = treeResourceNames.slice(1)
    const nextResourceName = subTreeResourceNames[0] ?? null
    const hasNext = !!nextResourceName

    const key = mapDataChildren(subTreeResourceNames[0] as ResourceName)
    const data: DynEntityType = useMemo(() => ({}), [])

    if (hasNext) {
        data[key] = (resource as any)[key]
    }

    const {lastCreatedResource, setLastCreatedResource} = useContext(NewResourceContext)

    useEffect(() => {
        if (lastCreatedResource?.id === resource.id) {
            setOpen(true)
            setLastCreatedResource(null)
        }
    }, [lastCreatedResource, setLastCreatedResource, resource.id])

    const menuItemRef = useRef<HTMLDivElement | null>(null)

    const [tenantId] = useTenantStore()

    let imageDocumentId = (resource as ElectionType).image_document_id ?? null

    const {data: imageData} = useGetOne("sequent_backend_document", {
        id: imageDocumentId,
        meta: {tenant_id: tenantId},
    })

    let item: React.ReactNode
    if (treeResourceNames[0] === "sequent_backend_election_event") {
        item = (
            <p className="flex items-center space-x-2">
                <HowToVoteIcon className="text-brand-color" />
                <span>{name}</span>
            </p>
        )
    } else if (imageData) {
        item = (
            <p className="flex items-center space-x-2">
                <img
                    width={24}
                    height={24}
                    src={`http://localhost:9000/public/tenant-${tenantId}/document-${imageDocumentId}/${imageData?.name}`}
                    alt={`tenant-${tenantId}/document-${imageDocumentId}/${imageData?.name}`}
                />
                <span>{name}</span>
            </p>
        )
    } else {
        item = <p>{name}</p>
    }

    return (
        <div className="bg-white">
            <div ref={menuItemRef} className="group flex text-left space-x-2 items-center">
                {canCreateElectionEvent ? (
                    <div className="flex-none w-6 h-6 cursor-pointer text-black" onClick={onClick}>
                        {open ? <ExpandMoreIcon /> : <ChevronRightIcon />}
                    </div>
                ) : (
                    <div className={cn(
                        "flex-none h-6",
                        canCreateElectionEvent && "w-6"
                    )}></div>
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
                        {item}
                    </NavLink>
                )}
                <div className="invisible group-hover:visible">
                    {canCreateElectionEvent
                        ? <MenuActions
                            isArchivedTab={isArchivedElectionEvents}
                            resourceId={id}
                            resourceName={name}
                            resourceType={treeResourceNames[0]}
                            parentData={superParentData}
                            menuItemRef={menuItemRef}
                        ></MenuActions>
                        : null
                    }
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
    const isEmpty =
        (!data?.electionEvents || data.electionEvents.length === 0) && isArchivedElectionEvents

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
                {isEmpty ? (
                    <div className="p-4 bg-white">No result</div>
                ) : (
                    <TreeLeaves
                        data={data}
                        parentData={data as DataTreeMenuType}
                        treeResourceNames={treeResourceNames}
                        isArchivedElectionEvents={isArchivedElectionEvents}
                    />
                )}
            </div>
        </>
    )
}
