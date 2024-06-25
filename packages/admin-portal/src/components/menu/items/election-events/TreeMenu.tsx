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
import {adminTheme, translate, translateElection} from "@sequentech/ui-essentials"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {Box} from "@mui/material"
import {MenuStyles} from "@/components/styles/Menu"

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
    const {t, i18n} = useTranslation()

    useEffect(() => {
        const dir = i18n.dir(i18n.language)
        document.documentElement.dir = dir
    }, [i18n, i18n.language, data])

    const {canCreateElectionEvent} = useActionPermissions()
    return (
        <Box sx={{backgroundColor: adminTheme.palette.white}}>
            <MenuStyles.TreeLeavesContainer>
                {data?.[mapDataChildren(treeResourceNames[0])]?.map(
                    (resource: DataTreeMenuType) => {
                        return (
                            <TreeMenuItem
                                key={resource.id}
                                resource={resource}
                                parentData={resource}
                                superParentData={parentData}
                                id={resource.id}
                                name={
                                    translateElection(resource, "alias", i18n.language) ||
                                    translateElection(resource, "name", i18n.language) ||
                                    resource.alias ||
                                    resource.name ||
                                    "-"
                                }
                                treeResourceNames={treeResourceNames}
                                isArchivedElectionEvents={isArchivedElectionEvents}
                                canCreateElectionEvent={canCreateElectionEvent}
                            />
                        )
                    }
                )}
                {!isArchivedElectionEvents && canCreateElectionEvent && (
                    <MenuStyles.CreateElectionContainer
                        style={{
                            justifyContent: i18n.dir(i18n.language) === "rtl" ? "end" : "start",
                        }}
                    >
                        <MenuStyles.StyledAddIcon
                            style={{
                                display: i18n.dir(i18n.language) === "rtl" ? "none" : "start",
                            }}
                        />
                        <MenuStyles.StyledNavLink
                            className={treeResourceNames[0]}
                            to={getNavLinkCreate(parentData, treeResourceNames[0])}
                            style={{textAlign: i18n.dir(i18n.language) === "rtl" ? "end" : "start"}}
                        >
                            {t(mapAddResource[treeResourceNames[0] as ResourceName])}
                        </MenuStyles.StyledNavLink>
                        <MenuStyles.StyledAddIcon
                            style={{
                                display: i18n.dir(i18n.language) === "rtl" ? "block" : "none",
                            }}
                        />
                        <MenuStyles.StyledHiddenDiv />
                    </MenuStyles.CreateElectionContainer>
                )}
            </MenuStyles.TreeLeavesContainer>
        </Box>
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
    const {i18n} = useTranslation()
    const {globalSettings} = useContext(SettingsContext)

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
            <MenuStyles.ItemContainer>
                <MenuStyles.HowToVoteStyledIcon />
                <span>{name}</span>
            </MenuStyles.ItemContainer>
        )
    } else if (imageData) {
        item = (
            <MenuStyles.ItemContainer>
                <img
                    width={24}
                    height={24}
                    src={`${globalSettings.PUBLIC_BUCKET_URL}tenant-${tenantId}/document-${imageDocumentId}/${imageData?.name}`}
                />
                <span>{name}</span>
            </MenuStyles.ItemContainer>
        )
    } else {
        item = <p>{name}</p>
    }

    return (
        <Box sx={{backgroundColor: adminTheme.palette.white}}>
            <MenuStyles.TreeMenuItemContainer ref={menuItemRef}>
                {hasNext && canCreateElectionEvent ? (
                    <MenuStyles.TreeMenuIconContaier onClick={onClick}>
                        {open ? (
                            <ExpandMoreIcon />
                        ) : (
                            <ChevronRightIcon
                                style={{
                                    transform:
                                        i18n.dir(i18n.language) === "rtl"
                                            ? "rotate(180deg)"
                                            : "rotate(0)",
                                }}
                            />
                        )}
                    </MenuStyles.TreeMenuIconContaier>
                ) : (
                    <MenuStyles.StyledDiv isWidth={canCreateElectionEvent} />
                )}
                {isOpenSidebar && (
                    <MenuStyles.StyledSideBarNavLink
                        title={name}
                        className={({isActive}) => (isActive ? "active" : "")}
                        to={`/${treeResourceNames[0]}/${id}`}
                        style={{textAlign: i18n.dir(i18n.language) === "rtl" ? "end" : "start"}}
                    >
                        {item}
                    </MenuStyles.StyledSideBarNavLink>
                )}
                <MenuStyles.MenuActionContainer className={`menu-actions-${treeResourceNames[0]}`}>
                    {canCreateElectionEvent ? (
                        <MenuActions
                            isArchivedTab={isArchivedElectionEvents}
                            resourceId={id}
                            resourceName={name}
                            resourceType={treeResourceNames[0]}
                            parentData={superParentData}
                            menuItemRef={menuItemRef}
                        ></MenuActions>
                    ) : null}
                </MenuStyles.MenuActionContainer>
            </MenuStyles.TreeMenuItemContainer>
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
        </Box>
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
    // console.log({data, treeResourceNames})
    const {t} = useTranslation()
    const isEmpty =
        (!data?.electionEvents || data.electionEvents.length === 0) && isArchivedElectionEvents
    return (
        <>
            <MenuStyles.SideMenuContainer>
                <MenuStyles.SideMenuActiveItem
                    onClick={() => onArchiveElectionEventsSelect(0)}
                    isArchivedElectionEvents={isArchivedElectionEvents}
                >
                    {t("sideMenu.active")}
                </MenuStyles.SideMenuActiveItem>
                <MenuStyles.SideMenuArchiveItem
                    onClick={() => onArchiveElectionEventsSelect(1)}
                    isArchivedElectionEvents={isArchivedElectionEvents}
                >
                    {t("sideMenu.archived")}
                </MenuStyles.SideMenuArchiveItem>
            </MenuStyles.SideMenuContainer>
            <Box sx={{paddingY: 1}}>
                {isEmpty ? (
                    <MenuStyles.EmptyStateContainer>No Result</MenuStyles.EmptyStateContainer>
                ) : (
                    <TreeLeaves
                        data={data}
                        parentData={data as DataTreeMenuType}
                        treeResourceNames={treeResourceNames}
                        isArchivedElectionEvents={isArchivedElectionEvents}
                    />
                )}
            </Box>
        </>
    )
}
