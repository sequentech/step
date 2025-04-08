// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useCallback, useContext, useEffect, useMemo, useRef, useState} from "react"
import {useLocation} from "react-router-dom"
import {useGetOne, useSidebarState} from "react-admin"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"
import ChevronRightIcon from "@mui/icons-material/ChevronRight"

import {
    mapDataChildren,
    ResourceName,
    DataTreeMenuType,
    DynEntityType,
    ElectionType,
    ContestType,
    CandidateType,
    TREE_RESOURCE_NAMES,
} from "../ElectionEvents"

import {useTranslation} from "react-i18next"

import MenuActions from "./MenuActions"
import {useActionPermissions} from "../use-tree-menu-hook"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {NewResourceContext} from "@/providers/NewResourceProvider"
import {adminTheme} from "@sequentech/ui-essentials"
import {translateElection} from "@sequentech/ui-core"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {Box, Menu, MenuItem} from "@mui/material"
import {MenuStyles, TreeMenuItemContainer} from "@/components/styles/Menu"
import {Sequent_Backend_Document} from "@/gql/graphql"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {useCreateElectionEventStore} from "@/providers/CreateElectionEventContextProvider"
import {useNavigate} from "react-router-dom"
import RefreshIcon from "@mui/icons-material/Refresh"

export const mapAddResource: Record<ResourceName, string> = {
    sequent_backend_election_event: "createResource.electionEvent",
    sequent_backend_election: "createResource.election",
    sequent_backend_contest: "createResource.contest",
    sequent_backend_candidate: "createResource.candidate",
}

export const mapImportResource: Record<ResourceName, string> = {
    sequent_backend_election_event: "importResource.electionEvent",
    sequent_backend_election: "importResource.election",
    sequent_backend_contest: "importResource.contest",
    sequent_backend_candidate: "importResource.candidate",
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
    reloadTree: () => void
}

function TreeLeaves({
    data,
    parentData,
    treeResourceNames,
    isArchivedElectionEvents,
    reloadTree,
}: TreeLeavesProps) {
    const {t, i18n} = useTranslation()
    const {openCreateDrawer, openImportDrawer} = useCreateElectionEventStore()
    const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null)

    useEffect(() => {
        const dir = i18n.dir(i18n.language)
        document.documentElement.dir = dir
    }, [i18n, i18n.language, data])

    /**
     * Permissions
     */

    const {canCreateElectionEvent, canCreateContest, canCreateElection, canCreateCandidate} =
        useActionPermissions()

    const canShowCreateMenu =
        (treeResourceNames[0] === "sequent_backend_election_event" && canCreateElectionEvent) ||
        (treeResourceNames[0] === "sequent_backend_election" && canCreateElection) ||
        (treeResourceNames[0] === "sequent_backend_contest" &&
            canCreateContest &&
            canCreateElection) ||
        (treeResourceNames[0] === "sequent_backend_candidate" &&
            canCreateCandidate &&
            canCreateElection &&
            canCreateContest)
    /**
     * ======
     */

    const handleOpenCreateElectionEventMenu = (e: React.MouseEvent<HTMLElement>) => {
        setAnchorEl(e.currentTarget)
    }

    const handleOpenCreateElectionEventForm = (e: React.MouseEvent<HTMLElement>) => {
        setAnchorEl(null)
        openCreateDrawer?.()
    }

    const handleOpenImportElectionEventForm = (e: React.MouseEvent<HTMLElement>) => {
        setAnchorEl(null)
        openImportDrawer?.()
    }

    /**
     * @description
     * Given a resource, traverse all its children (elections, contests, candidates)
     * and return an array of all the ids of the children to reopen the tree.
     *
     * @param {DataTreeMenuType} resource - The resource to traverse.
     * @returns {Array<string>} - An array of all the ids of the children.
     */
    const fillPath = useCallback((resource: DataTreeMenuType) => {
        const allIds = []
        allIds.push(resource.id)
        if ("elections" in resource) {
            for (let election of resource.elections as ElectionType[]) {
                allIds.push(election.id)
                for (let contest of election.contests as ContestType[]) {
                    allIds.push(contest.id)
                    for (let candidate of contest.candidates as CandidateType[]) {
                        allIds.push(candidate.id)
                    }
                }
            }
        } else if ("contests" in resource) {
            for (let contest of resource.contests as ContestType[]) {
                allIds.push(contest.id)
                for (let candidate of contest.candidates) {
                    allIds.push(candidate.id)
                }
            }
        } else if ("candidates" in resource && resource.candidates !== null) {
            for (let candidate of resource.candidates as CandidateType[]) {
                allIds.push(candidate.id)
            }
        }
        return allIds
    }, [])

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
                                fullPath={fillPath(resource)}
                                reloadTree={reloadTree}
                            />
                        )
                    }
                )}
                {!isArchivedElectionEvents && canShowCreateMenu && (
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

                        {treeResourceNames[0] === TREE_RESOURCE_NAMES[0] ? (
                            <MenuStyles.StyledNavLinkButton
                                className={treeResourceNames[0]}
                                style={{
                                    textAlign: i18n.dir(i18n.language) === "rtl" ? "end" : "start",
                                }}
                                onClick={handleOpenCreateElectionEventMenu}
                            >
                                {t(mapAddResource[treeResourceNames[0] as ResourceName])}
                            </MenuStyles.StyledNavLinkButton>
                        ) : (
                            <MenuStyles.StyledNavLink
                                className={treeResourceNames[0]}
                                to={getNavLinkCreate(parentData, treeResourceNames[0])}
                                style={{
                                    textAlign: i18n.dir(i18n.language) === "rtl" ? "end" : "start",
                                }}
                            >
                                {t(mapAddResource[treeResourceNames[0] as ResourceName])}
                            </MenuStyles.StyledNavLink>
                        )}

                        <MenuStyles.StyledAddIcon
                            style={{
                                display: i18n.dir(i18n.language) === "rtl" ? "block" : "none",
                            }}
                        />
                        <MenuStyles.StyledHiddenDiv />
                    </MenuStyles.CreateElectionContainer>
                )}
            </MenuStyles.TreeLeavesContainer>
            <Menu
                id="treemenu-create-election-event-menu"
                anchorEl={anchorEl}
                anchorOrigin={{
                    vertical: "bottom",
                    horizontal: "left",
                }}
                keepMounted
                transformOrigin={{
                    vertical: "top",
                    horizontal: "right",
                }}
                open={Boolean(anchorEl)}
                onClose={() => setAnchorEl(null)}
            >
                <MenuItem className="menu-sidebar-item" onClick={handleOpenCreateElectionEventForm}>
                    <Box
                        sx={{
                            textOverflow: "ellipsis",
                            whiteSpace: "nowrap",
                            overflow: "hidden",
                        }}
                    >
                        <span className="help-menu-item" title={"Create Election Event"}>
                            {t("createResource.electionEvent")}
                        </span>
                    </Box>
                </MenuItem>
                <MenuItem className="menu-sidebar-item" onClick={handleOpenImportElectionEventForm}>
                    <Box
                        sx={{
                            textOverflow: "ellipsis",
                            whiteSpace: "nowrap",
                            overflow: "hidden",
                        }}
                    >
                        <span className="help-menu-item" title={"Import Election Event"}>
                            {t("electionEventScreen.import.eetitle")}
                        </span>
                    </Box>
                </MenuItem>
            </Menu>
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
    fullPath: string[] | null | undefined
    reloadTree: () => void
}

function TreeMenuItem({
    resource,
    parentData,
    superParentData,
    id,
    name,
    treeResourceNames,
    isArchivedElectionEvents,
    fullPath,
    reloadTree,
}: TreeMenuItemProps) {
    const [isOpenSidebar] = useSidebarState()
    const {i18n} = useTranslation()
    const {globalSettings} = useContext(SettingsContext)

    const [open, setOpen] = useState(false)

    const location = useLocation()
    const {setTallyId, setTaskId, setCustomFilter} = useElectionEventTallyStore()

    const onClick = (isLabel: boolean) => {
        if (isLabel && open) {
            return
        }
        if (!open) {
            reloadTree()
        }
        setOpen(!open)
    }
    /**
     * control the tree menu open state
     */
    useEffect(() => {
        // set context tally to null to allow navigation to new election event tally
        setTallyId(null)
        // set context task to null to allow navigation to new election event task
        setTaskId(null)
        // set context task to null to allow navigation to new election event task
        setCustomFilter({})

        // open menu on url navigation or paste
        setTimeout(() => {
            for (const id of fullPath ?? []) {
                if (id === resource.id) {
                    setOpen(true)
                }
            }
        }, 400)
    }, [location.pathname])

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
    const [anchorEl, setAnchorEl] = React.useState<HTMLParagraphElement | null>(null)
    const isClicked = anchorEl ? true : false

    const [tenantId] = useTenantStore()

    let imageDocumentId = (resource as ElectionType).image_document_id ?? null

    const {data: imageData} = useGetOne<Sequent_Backend_Document>(
        "sequent_backend_document",
        {
            id: imageDocumentId,
            meta: {tenant_id: tenantId},
        },
        {
            enabled: !!imageDocumentId && !!tenantId,
            onError: (error: any) => {
                console.log(`error fetching image doc: ${error.message}`)
            },
            onSuccess: () => {
                console.log(`success fetching image doc`)
            },
        }
    )

    let item: React.ReactNode
    if (treeResourceNames[0] === "sequent_backend_election_event") {
        item = (
            <MenuStyles.ItemContainer>
                <MenuStyles.HowToVoteStyledIcon />
                <MenuStyles.SpanContainer>{name}</MenuStyles.SpanContainer>
            </MenuStyles.ItemContainer>
        )
    } else if (imageData) {
        item = (
            <MenuStyles.ItemContainer>
                <img
                    alt={name}
                    width={24}
                    height={24}
                    src={`${globalSettings.PUBLIC_BUCKET_URL}tenant-${tenantId}/document-${imageDocumentId}/${imageData?.name}`}
                />
                <MenuStyles.SpanContainer>{name}</MenuStyles.SpanContainer>
            </MenuStyles.ItemContainer>
        )
    } else {
        item = (
            <MenuStyles.ItemContainer>
                <MenuStyles.SpanContainer>{name}</MenuStyles.SpanContainer>
            </MenuStyles.ItemContainer>
        )
    }

    /**
     * Permissions
     */
    const {canCreateElectionEvent, canReadContest, canReadCandidate, canReadElection} =
        useActionPermissions()

    const canShowMenu =
        (hasNext && treeResourceNames[0] === "sequent_backend_election_event" && canReadElection) ||
        (hasNext && treeResourceNames[0] === "sequent_backend_election" && canReadContest) ||
        (hasNext && treeResourceNames[0] === "sequent_backend_contest" && canReadCandidate) ||
        (hasNext && treeResourceNames[0] === "sequent_backend_candidate")

    /**
     * ======
     */

    return (
        <Box sx={{backgroundColor: adminTheme.palette.white}}>
            <TreeMenuItemContainer ref={menuItemRef} isClicked={isClicked}>
                {canShowMenu ? (
                    <MenuStyles.TreeMenuIconContaier onClick={() => onClick(false)}>
                        {resource?.active && open ? (
                            <ExpandMoreIcon className="menu-item-expanded" />
                        ) : (
                            <ChevronRightIcon
                                className="menu-item-collapsed"
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
                        multiline={
                            treeResourceNames[0] === "sequent_backend_election" ? "true" : undefined
                        } // Fix here
                        onClick={() => onClick(true)}
                        title={name}
                        className={({isActive}) =>
                            isActive ? `active menu-item-${treeResourceNames[0]}` : ``
                        }
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
                            setAnchorEl={setAnchorEl}
                            anchorEl={anchorEl}
                            reloadTree={reloadTree}
                        ></MenuActions>
                    ) : null}
                </MenuStyles.MenuActionContainer>
            </TreeMenuItemContainer>
            {resource?.active && open && (
                <div className="">
                    {hasNext && (
                        <TreeLeaves
                            data={data}
                            parentData={parentData}
                            treeResourceNames={subTreeResourceNames}
                            isArchivedElectionEvents={isArchivedElectionEvents}
                            reloadTree={reloadTree}
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
    reloadTree,
}: {
    data: DynEntityType
    treeResourceNames: ResourceName[]
    isArchivedElectionEvents: boolean
    onArchiveElectionEventsSelect: (val: number) => void
    reloadTree: () => void
}) {
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
                {/* TODO: not working well
                <MenuStyles.RefreshAction>
                    <RefreshIcon onClick={reloadTree} />
                </MenuStyles.RefreshAction>*/}
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
                        reloadTree={reloadTree}
                    />
                )}
            </Box>
        </>
    )
}
