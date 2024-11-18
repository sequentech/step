// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useMemo, useRef, useState} from "react"
import {NavLink, useLocation} from "react-router-dom"
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
    ElectionEventType,
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
import {
    Sequent_Backend_Document,
    Sequent_Backend_Tasks_Execution_Update_Column,
} from "@/gql/graphql"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {useCreateElectionEventStore} from "@/providers/CreateElectionEventContextProvider"

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
    const {toggleImportDrawer, openCreateDrawer} = useCreateElectionEventStore()
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
        console.log({e})
        setAnchorEl(null)
        openCreateDrawer?.()
    }

    const handleOpenImportElectionEventForm = (e: React.MouseEvent<HTMLElement>) => {
        console.log({e})
        setAnchorEl(null)
        toggleImportDrawer?.((prev) => !prev)
    }

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
}

function TreeMenuItem({
    resource,
    parentData,
    superParentData,
    id,
    name,
    treeResourceNames,
    isArchivedElectionEvents,
}: TreeMenuItemProps) {
    const [isOpenSidebar] = useSidebarState()
    const {i18n} = useTranslation()
    const {globalSettings} = useContext(SettingsContext)

    const [open, setOpen] = useState(resource.isActive)
    // const [isFirstLoad, setIsFirstLoad] = useState(true)

    const location = useLocation()
    const {setTallyId, setTaskId, setCustomFilter} = useElectionEventTallyStore()

    const onClick = () => setOpen(!open)

    useEffect(() => {
        // set context tally to null to allow navigation to new election event tally
        setTallyId(null)
        // set context task to null to allow navigation to new election event task
        setTaskId(null)
        // set context task to null to allow navigation to new election event task
        setCustomFilter({})
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

    const {data: imageData} = useGetOne<Sequent_Backend_Document>("sequent_backend_document", {
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
                    <MenuStyles.TreeMenuIconContaier onClick={onClick}>
                        {open ? (
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
                        ></MenuActions>
                    ) : null}
                </MenuStyles.MenuActionContainer>
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

type NavEntityType =
    | "sequent_backend_election_event"
    | "sequent_backend_election"
    | "sequent_backend_contest"
    | "sequent_backend_candidate"

type NavEntityKey = "election_event_id" | "candidate_id" | "contest_id" | "election_id"

const findNavItem = ({d, id, entity}: {d: DynEntityType; id: string; entity: NavEntityType}) => {
    if (entity === "sequent_backend_election_event") {
        return d.electionEvents?.find((el: ElectionEventType) => el.id === id)
    }

    const flatElections = d.electionEvents?.flatMap((event: ElectionEventType) => event.elections)
    if (entity === "sequent_backend_election") {
        return flatElections?.find((el: ElectionType) => el.id === id)
    }

    const flatContests = flatElections?.flatMap((election: ElectionType) => election.contests)
    if (entity === "sequent_backend_contest") {
        return flatContests?.find((el: ContestType) => el.id === id)
    }

    const flatCandidates = flatContests?.flatMap((contest: ContestType) => contest.candidates)
    if (entity === "sequent_backend_candidate") {
        return flatCandidates?.find((el: CandidateType) => el.id === id)
    }
}

const isValidId = (id: string) => {
    const uuidRegex = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i
    return uuidRegex.test(id)
}

const getIdFromUrl = (url: string) => {
    //search for entityId via uuid pattern match
    let uuidPattern =
        /[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-5][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}/

    let match = url.match(uuidPattern)

    if (match) {
        let uuid = match[0]
        console.log(uuid)
        return uuid
    } else {
        console.log("UUID not found in the URL.")
        return null
    }
}

const getUrlEntity = (
    url: string
): {type: NavEntityType; key: NavEntityKey; id: string | null} | null => {
    if (url.includes("sequent_backend_election_event")) {
        return {
            type: "sequent_backend_election_event",
            key: "election_event_id",
            id: getIdFromUrl(url),
        }
    }
    if (url.includes("sequent_backend_candidate")) {
        return {id: getIdFromUrl(url), type: "sequent_backend_candidate", key: "candidate_id"}
    }
    if (url.includes("sequent_backend_contest")) {
        return {id: getIdFromUrl(url), type: "sequent_backend_contest", key: "contest_id"}
    }
    if (url.includes("sequent_backend_election")) {
        return {id: getIdFromUrl(url), type: "sequent_backend_election", key: "election_id"}
    } else {
        return null
    }
}

const updateTreeData = ({
    data,
    entityDetails,
}: {
    data: DynEntityType
    entityDetails: any
}): DynEntityType => {
    return {
        ...data,
        //@ts-ignore
        electionEvents: entityDetails.election_event_id
            ? data.electionEvents?.map((event: ElectionEventType) => {
                  if (event.id === entityDetails.election_event_id) {
                      return {
                          ...event,
                          isActive: true,
                          elections: entityDetails.election_id
                              ? event.elections.map((election: ElectionType) => {
                                    if (election.id === entityDetails.election_id) {
                                        return {
                                            ...election,
                                            isActive: true,
                                            contests: entityDetails.contest_id
                                                ? election.contests.map((contest: ContestType) => {
                                                      if (contest.id === entityDetails.contest_id) {
                                                          return {
                                                              ...contest,
                                                              isActive: true,
                                                              candidates: entityDetails.candidate_id
                                                                  ? contest.candidates.map(
                                                                        (
                                                                            candidate: CandidateType
                                                                        ) => {
                                                                            if (
                                                                                candidate.id ===
                                                                                entityDetails.candidate_id
                                                                            ) {
                                                                                return {
                                                                                    ...candidate,
                                                                                    isActive: true,
                                                                                }
                                                                            }
                                                                            return candidate
                                                                        }
                                                                    )
                                                                  : contest.candidates,
                                                          }
                                                      }
                                                      return contest
                                                  })
                                                : election.contests,
                                        }
                                    }
                                    return election
                                })
                              : event.elections,
                      }
                  }
                  return event
              })
            : data.electionEvents,
    }
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
    const location = useLocation()
    const {t} = useTranslation()
    const isEmpty =
        (!data?.electionEvents || data.electionEvents.length === 0) && isArchivedElectionEvents

    const updatedData = useMemo(() => {
        const entityConfig = getUrlEntity(window.location.href)

        if (!entityConfig?.id) return data

        const aItem = findNavItem({d: data, id: entityConfig.id, entity: entityConfig.type})

        if (!aItem) return data

        //@ts-ignore //ignored because its a temporal key used to update the tree data
        aItem[entityConfig.key] = aItem?.id
        return updateTreeData({data, entityDetails: aItem})
    }, [location.pathname, data])

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
                        data={updatedData}
                        parentData={updatedData as DataTreeMenuType}
                        treeResourceNames={treeResourceNames}
                        isArchivedElectionEvents={isArchivedElectionEvents}
                    />
                )}
            </Box>
        </>
    )
}
