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
import {translate, translateElection} from "@sequentech/ui-essentials"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import styled from "@emotion/styled"
import {colors} from '../../../../constants/colors'
import { Box } from "@mui/material"

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
    console.log('treeResourcesName', treeResourceNames[0]);
    return (
        <Box sx={{backgroundColor: colors.white}}>
            <TreeLeavesContainer>
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
                    <CreateElectionContainer
                        style={{
                            justifyContent: i18n.dir(i18n.language) === "rtl" ? "end" : "start",
                        }}
                    >
                        <StyledAddIcon
                            style={{
                                display: i18n.dir(i18n.language) === "rtl" ? "none" : "start",
                            }}
                        />
                        <StyledNavLink
                            className={treeResourceNames[0]}
                            to={getNavLinkCreate(parentData, treeResourceNames[0])}
                            style={{textAlign: i18n.dir(i18n.language) === "rtl" ? "end" : "start"}}
                        >
                            {t(mapAddResource[treeResourceNames[0] as ResourceName])}
                        </StyledNavLink>
                        <StyledAddIcon
                            style={{
                                display: i18n.dir(i18n.language) === "rtl" ? "block" : "none",
                            }}
                        />
                        <StyledHiddenDiv/>
                    </CreateElectionContainer>
                )}
            </TreeLeavesContainer>
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
                    src={`${globalSettings.PUBLIC_BUCKET_URL}tenant-${tenantId}/document-${imageDocumentId}/${imageData?.name}`}
                />
                <span>{name}</span>
            </p>
        )
    } else {
        item = <p>{name}</p>
    }

    return (
        <Box sx={{backgroundColor: colors.white}}>

            <TreeMenuItemContainer ref={menuItemRef}>
                {hasNext && canCreateElectionEvent ? (
                    <div className="flex-none w-6 h-6 cursor-pointer text-black" onClick={onClick}>
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
                    </div>
                ) : (
                    <div className={cn("flex-none h-6", canCreateElectionEvent && "w-6")}></div>
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
                        style={{textAlign: i18n.dir(i18n.language) === "rtl" ? "end" : "start"}}
                    >
                        {item}
                    </NavLink>
                )}
                <div
                    className={`invisible group-hover:visible menu-actions-${treeResourceNames[0]}`}
                >
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
                </div>
            </TreeMenuItemContainer>
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
    const {t} = useTranslation()
    const isEmpty =
        (!data?.electionEvents || data.electionEvents.length === 0) && isArchivedElectionEvents
        console.log('isEmpty', isEmpty);
    return (
        <>
           <SideMenuContainer>
            <SideMenuActiveItem onClick={() => onArchiveElectionEventsSelect(0)} isArchivedElectionEvents = {isArchivedElectionEvents}>
                {t("sideMenu.active")}
            </SideMenuActiveItem>
            <SideMenuArchiveItem onClick={() => onArchiveElectionEventsSelect(1)} isArchivedElectionEvents = {isArchivedElectionEvents}>
                {t("sideMenu.archived")}
            </SideMenuArchiveItem>
            </SideMenuContainer>
            <Box sx={{paddingY: 1}}>
                {isEmpty ? (
                    <EmptyStateContainer>
                        No Result
                    </EmptyStateContainer>
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

const SideMenuContainer = styled.ul`
  display: flex;
  padding-left: 1rem;
  padding-right: 1rem;
  background-color: white;
  text-transform: uppercase;
  font-size: 0.75rem;
  line-height: 1.5rem;
  & > *:not(:last-child) {
    margin-right: 1rem;
  }
`;

const SideMenuActiveItem = styled.li<{isArchivedElectionEvents: boolean}>`
padding-left: 1rem;
padding-right: 1rem;
padding-top: 0.5rem;
padding-bottom: 0.5rem;
cursor: pointer;
color: ${({isArchivedElectionEvents}) => (!isArchivedElectionEvents ? colors.brandColor : colors.secondary)};
border-bottom: ${({ isArchivedElectionEvents }) => (!isArchivedElectionEvents ? `2px solid ${colors.brandSuccess}` : 'none')};
`

const SideMenuArchiveItem = styled.li<{isArchivedElectionEvents: boolean}>`
padding-left: 1rem;
padding-right: 1rem;
padding-top: 0.5rem;
padding-bottom: 0.5rem;
cursor: pointer;
color: ${({isArchivedElectionEvents}) => (isArchivedElectionEvents ? colors.brandColor : colors.secondary)};
border-bottom: ${({ isArchivedElectionEvents }) => (isArchivedElectionEvents ? `2px solid ${colors.brandSuccess}` : 'none')};
`

const EmptyStateContainer = styled.div`
  padding: 1rem;
  background-color: white;
`

const TreeLeavesContainer = styled.div`
    display: flex;
    flex-direction: column;
    margin-left: 0.75rem;
`

const CreateElectionContainer = styled.div`
display: flex;
align-items: center;
color: ${colors.secondary};
& > *:not(:last-child) {
  margin-right: 0.5rem;
}
`

const StyledAddIcon = styled(AddIcon)`
flex: 0 0 auto;
`

const StyledNavLink = styled(NavLink)`
flex-grow: 1;
padding-top: 0.375rem;
padding-bottom: 0.375rem;
border-bottom-width: 2px;
border-bottom-color: white;
cursor: pointer;
white-space: nowrap;
overflow: hidden;
text-overflow: ellipsis;

&:hover {
  border-bottom-color: ${colors.secondary};
}
`

const StyledHiddenDiv = styled.div`
flex: 0 0 auto;
width: 1.5rem;
height: 1.5rem;
visibility: hidden;
`

const TreeMenuItemContainer = styled.div`
  display: flex;                   /* flex */
  text-align: left;                /* text-left */
  align-items: center;             /* items-center */
  & > *:not(:last-child) {
    margin-right: 0.5rem;          /* space-x-2 */
  }
`;


