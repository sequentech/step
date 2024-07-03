// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useState} from "react"
import {useAtom} from "jotai"
import archivedElectionEventSelection from "@/atoms/archived-election-event-selection"
import {useLocation} from "react-router-dom"
import styled from "@emotion/styled"
import {IconButton, adminTheme} from "@sequentech/ui-essentials"
import {
    ICandidatePresentation,
    IContestPresentation,
    IElectionEventPresentation,
    IContest,
} from "@sequentech/ui-core"
import SearchIcon from "@mui/icons-material/Search"
import {CircularProgress, TextField} from "@mui/material"
import {Menu, useSidebarState} from "react-admin"
import {TreeMenu} from "./election-events/TreeMenu"
import {faPlusCircle} from "@fortawesome/free-solid-svg-icons"
import WebIcon from "@mui/icons-material/Web"
import {HorizontalBox} from "../../HorizontalBox"
import {Link} from "react-router-dom"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTranslation} from "react-i18next"
import {IPermissions} from "../../../types/keycloak"
import {useTreeMenuData} from "./use-tree-menu-hook"
import {cloneDeep} from "lodash"
import {sortCandidatesInContest, sortContestList, sortElectionList} from "@sequentech/ui-core"

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

type BaseType = {__typename: ResourceName; id: string; name: string; alias?: string}

export type CandidateType = BaseType & {
    __typename: "sequent_backend_candidate"
    election_event_id: string
    contest_id: string
    presentation: ICandidatePresentation
}

export type ContestType = BaseType &
    IContest & {
        __typename: "sequent_backend_contest"
        election_event_id: string
        election_id: string
        presentation: IContestPresentation
        candidates: Array<CandidateType>
    }

export type ElectionType = BaseType & {
    __typename: "sequent_backend_election"
    election_event_id: string
    image_document_id: string
    contests: Array<ContestType>
}

export type ElectionEventType = BaseType & {
    __typename: "sequent_backend_election_event"
    is_archived: boolean
    elections: Array<ElectionType>
    presentation: IElectionEventPresentation
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
    const [isArchivedElectionEvents, setArchivedElectionEvents] = useAtom(
        archivedElectionEventSelection
    )
    const authContext = useContext(AuthContext)
    const showAddElectionEvent = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.ELECTION_EVENT_CREATE
    )
    const {t, i18n} = useTranslation()

    const {data, loading} = useTreeMenuData(isArchivedElectionEvents)

    function handleSearchChange(searchInput: string) {
        setSearchInput(searchInput)
    }

    function changeArchiveSelection(val: number) {
        setArchivedElectionEvents(val === 1)
    }

    const location = useLocation()
    const isElectionEventActive = TREE_RESOURCE_NAMES.some(
        (route) => location.pathname.search(route) > -1
    )

    let resultData = data
    if (!loading && data && data.sequent_backend_election_event) {
        resultData = filterTree({electionEvents: data?.sequent_backend_election_event}, searchInput)
    }

    resultData = {
        electionEvents: cloneDeep(resultData?.electionEvents ?? [])?.map(
            (electionEvent: ElectionEventType) => {
                const electionOrderType = electionEvent?.presentation?.elections_order
                return {
                    ...electionEvent,
                    elections: sortElectionList(electionEvent.elections, electionOrderType).map(
                        (election: any) => {
                            const contestOrderType = election?.presentation?.contests_order
                            return {
                                ...election,
                                contests: sortContestList(election.contests, contestOrderType).map(
                                    (contest: any) => {
                                        let orderType = contest.presentation?.candidates_order

                                        contest.candidates = sortCandidatesInContest(
                                            contest.candidates,
                                            orderType
                                        ) as any

                                        return contest
                                    }
                                ),
                            }
                        }
                    ),
                }
            }
        ),
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
            <Container isActive={isElectionEventActive}>
                <HorizontalBox
                    sx={{
                        alignItems: "center",
                        paddingRight: i18n.dir(i18n.language) === "rtl" ? 0 : "16px",
                        paddingLeft: i18n.dir(i18n.language) === "rtl" ? "32px" : 0,
                    }}
                >
                    <MenuItem
                        to="/sequent_backend_election_event"
                        primaryText={isOpenSidebar && t("sideMenu.electionEvents")}
                        leftIcon={<WebIcon sx={{color: adminTheme.palette.brandColor}} />}
                        sx={{
                            flexGrow: 2,
                            paddingLeft: i18n.dir(i18n.language) === "rtl" ? 0 : "16px",
                            paddingRight: i18n.dir(i18n.language) === "rtl" ? "16px" : 0,
                        }}
                    />
                    {isOpenSidebar && showAddElectionEvent ? (
                        <Link to="/sequent_backend_election_event/create">
                            <StyledIconButton
                                className="election-event-create-button"
                                icon={faPlusCircle}
                                size="xs"
                            />
                        </Link>
                    ) : null}
                </HorizontalBox>

                {isOpenSidebar && (
                    <>
                        <SideBarContainer dir={i18n.dir(i18n.language)}>
                            <TextField
                                dir={i18n.dir(i18n.language)}
                                label={t("sideMenu.search")}
                                size="small"
                                value={searchInput}
                                onChange={(e) => handleSearchChange(e.target.value)}
                            />
                            <SearchIcon />
                        </SideBarContainer>

                        {treeMenu}
                    </>
                )}
            </Container>
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
    font-size: 1rem;
    line-height: 1.5rem;
`

const Container = styled("div")<{isActive?: boolean}>`
    background-color: ${({isActive}) => (isActive ? adminTheme.palette.green.light : "initial")};
`

const SideBarContainer = styled("div")`
    display: flex;
    align-items: center;
    background-color: white;
    padding-left: 1rem;
    padding-right: 1rem;
    & > *:not(:last-child) {
        margin-right: 1rem;
    }
`
