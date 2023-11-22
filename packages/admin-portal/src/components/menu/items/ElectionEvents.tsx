// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useState} from "react"

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
import {FETCH_ELECTION_EVENTS_TREE} from "../../../queries/get-election-events-tree"

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

const treeResourceNames = [
    "sequent_backend_election_event",
    "sequent_backend_election",
    "sequent_backend_contest",
    "sequent_backend_candidate",
]

export interface CandidatesTree {
    id: string
    name: string
}
export interface ContestTree {
    id: string
    name: string
    candidates: CandidatesTree[]
}
export interface ElectionTree {
    id: string
    name: string
    contests: ContestTree[]
}

export interface ElectionEventsTree {
    id: string
    name: string
    is_archived: boolean
    elections: ElectionTree[]
}

export default function ElectionEvents() {
    const [isOpenSidebar] = useSidebarState()
    const [searchInput, setSearchInput] = useState<string>("")
    const [archivedElectionEvents, setArchivedElectionEvents] = useState(0)

    const isArchivedElectionEvents = archivedElectionEvents === 1
    function handleSearchChange(searchInput: string) {
        setSearchInput(searchInput)
    }
    function changeArchiveSelection(val: number) {
        setArchivedElectionEvents(val)
    }

    const location = useLocation()
    const isElectionEventActive = treeResourceNames.some(
        (route) => location.pathname.search(route) > -1
    )

    const {data, loading, error} = useQuery(FETCH_ELECTION_EVENTS_TREE, {
        variables: {
            isArchived: isArchivedElectionEvents,
        },
    })

    const treeMenu = loading ? (
        <CircularProgress />
    ) : (
        <TreeMenu
            data={{electionEvents: data.sequent_backend_election_event}}
            treeResourceNames={treeResourceNames}
            isArchivedElectionEvents={isArchivedElectionEvents}
            onArchiveElectionEventsSelect={changeArchiveSelection}
            searchFilter={searchInput.trim()}
        />
    )

    return (
        <>
            <div className={cn(isElectionEventActive && "bg-green-light")}>
                <HorizontalBox sx={{alignItems: "center"}}>
                    <MenuItem
                        to="/sequent_backend_election_event"
                        primaryText={isOpenSidebar && "Election Events"}
                        leftIcon={<IconButton icon={faThLarge} fontSize="24px" />}
                        sx={{flexGrow: 2}}
                    />
                    <Link to="/sequent_backend_election_event/create">
                        <StyledIconButton icon={faPlusCircle} size="xs" />
                    </Link>
                </HorizontalBox>
                {isOpenSidebar && isElectionEventActive && (
                    <>
                        <div className="flex bg-white px-4">
                            <TextField
                                label="Search"
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
