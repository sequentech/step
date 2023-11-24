// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useState} from "react"
import {useQuery} from "@apollo/client"
import {useLocation} from "react-router-dom"
import {styled} from "@mui/material/styles"
import {IconButton, adminTheme} from "@sequentech/ui-essentials"
import {CircularProgress, TextField} from "@mui/material"
import {Menu, useSidebarState} from "react-admin"
import {TreeMenu} from "./election-events/TreeMenu"
import {faThLarge, faSearch, faPlusCircle} from "@fortawesome/free-solid-svg-icons"
import {cn} from "../../../lib/utils"
import {HorizontalBox} from "../../HorizontalBox"
import {Link} from "react-router-dom"
import {FETCH_ELECTION_EVENTS_TREE} from "../../../queries/GetElectionEventsTree"
import {useTenantStore} from "../../../providers/TenantContextProvider"
//import { IPermissions } from "sequent-core"
import {AuthContext} from "../../../providers/AuthContextProvider"
import {useTranslation} from "react-i18next"

export type ResourceName =
    | "sequent_backend_election_event"
    | "sequent_backend_election"
    | "sequent_backend_contest"
    | "sequent_backend_candidate"

export type EntityFieldName = "electionEvents" | "elections" | "contests" | "candidates"

export function mapDataChildren(key: ResourceName): EntityFieldName {
    const map: Record<ResourceName, EntityFieldName> = {
        sequent_backend_election_event: "electionEvents",
        sequent_backend_election: "elections",
        sequent_backend_contest: "contests",
        sequent_backend_candidate: "candidates",
    }
    return map[key]
}

const TREE_RESOURCE_NAMES: Array<ResourceName> = [
    "sequent_backend_election_event",
    "sequent_backend_election",
    "sequent_backend_contest",
    "sequent_backend_candidate",
]

const ENTITY_FIELD_NAMES: Array<EntityFieldName> = [
    "electionEvents",
    "elections",
    "contests",
    "candidates",
]

export interface CandidatesTree {
    id: string
    name: string
}
export interface ContestTree {
    id: string
    name: string
    candidates: Array<CandidatesTree>
}
export interface ElectionTree {
    id: string
    name: string
    contests: Array<ContestTree>
}

export interface ElectionEventsTree {
    id: string
    name: string
    is_archived: boolean
    elections: Array<ElectionTree>
}

type BaseType = {__typename: ResourceName; id: string; name: string}

type CandidateType = BaseType & {__typename: "sequent_backend_candidate"}

type ContestType = BaseType & {
    __typename: "sequent_backend_contest"
    candidates: Array<CandidateType>
}

type ElectionType = BaseType & {
    __typename: "sequent_backend_election"
    contests: Array<ContestType>
}

type ElectionEventType = BaseType & {
    __typename: "sequent_backend_election_event"
    is_archived: boolean
    elections: Array<ElectionType>
}

export type DynEntityType = {
    electionEvents?: ElectionEventType[]
    elections?: ElectionType[]
    contests?: ContestType[]
    candidates?: CandidateType[]
}

export type DataTreeMenuType = BaseType | CandidateType | ElectionType | ElectionEventType

function filterTree(tree: any, filterName: string): any {
    if (Array.isArray(tree)) {
        return tree.map((subTree) => filterTree(subTree, filterName)).filter((v) => v !== null)
    } else if (typeof tree === "object" && tree !== null) {
        for (let key in tree) {
            if (tree.name?.toLowerCase().search(filterName.toLowerCase()) > -1) {
                return tree
            } else if (ENTITY_FIELD_NAMES.includes(key as EntityFieldName)) {
                let filteredSubTree = filterTree(tree[key], filterName)
                if (filteredSubTree.length > 0) {
                    let filteredObj = {...tree}
                    filteredObj[key] = filteredSubTree
                    return filteredObj
                }
            }
        }
    }

    return null
}

export default function ElectionEvents() {
    const [tenantId] = useTenantStore()
    const [isOpenSidebar] = useSidebarState()
    const [searchInput, setSearchInput] = useState<string>("")
    const [archivedElectionEvents, setArchivedElectionEvents] = useState(0)
    const authContext = useContext(AuthContext)
    const showAddElectionEvent = authContext.isAuthorized(true, tenantId, "election-event-create")
    const {t} = useTranslation()

    const isArchivedElectionEvents = archivedElectionEvents === 1
    function handleSearchChange(searchInput: string) {
        setSearchInput(searchInput)
    }
    function changeArchiveSelection(val: number) {
        setArchivedElectionEvents(val)
    }

    const location = useLocation()
    const isElectionEventActive = TREE_RESOURCE_NAMES.some(
        (route) => location.pathname.search(route) > -1
    )

    const {data, loading} = useQuery(FETCH_ELECTION_EVENTS_TREE, {
        variables: {
            tenantId: tenantId,
            isArchived: isArchivedElectionEvents,
        },
    })

    let resultData = data
    if (!loading && data && data.sequent_backend_election_event) {
        resultData = filterTree({electionEvents: data?.sequent_backend_election_event}, searchInput)
    }

    const treeMenu = loading ? (
        <CircularProgress />
    ) : (
        <TreeMenu
            data={resultData}
            treeResourceNames={TREE_RESOURCE_NAMES}
            isArchivedElectionEvents={isArchivedElectionEvents}
            onArchiveElectionEventsSelect={changeArchiveSelection}
        />
    )

    return (
        <>
            <div className={cn(isElectionEventActive && "bg-green-light")}>
                <HorizontalBox sx={{alignItems: "center"}}>
                    <MenuItem
                        to="/sequent_backend_election_event"
                        primaryText={isOpenSidebar && t("sideMenu.electionEvents")}
                        leftIcon={<IconButton icon={faThLarge} fontSize="24px" />}
                        sx={{flexGrow: 2}}
                    />
                    {showAddElectionEvent ? (
                        <Link to="/sequent_backend_election_event/create">
                            <StyledIconButton icon={faPlusCircle} size="xs" />
                        </Link>
                    ) : null}
                </HorizontalBox>
                {isOpenSidebar && isElectionEventActive && (
                    <>
                        <div className="flex bg-white px-4">
                            <TextField
                                label={t("sideMenu.search")}
                                size="small"
                                value={searchInput}
                                onChange={(e) => handleSearchChange(e.target.value)}
                            />
                            <IconButton icon={faSearch} fontSize="18px" sx={{margin: "0 12px"}} />
                        </div>

                        {treeMenu}
                    </>
                )}
            </div>
        </>
    )
}

const MenuItem = styled(Menu.Item)`
    color: ${adminTheme.palette.brandColor};

    &.RaMenuItemLink-active,
    .MuiIconButton-root {
        color: ${adminTheme.palette.brandColor};
    }
`

const StyledIconButton = styled(IconButton)`
    &:hover {
        padding: unset !important;
    }
    margin-right: 16px;
    font-size: 1rem;
    line-height: 1.5rem;
`
