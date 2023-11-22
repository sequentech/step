// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useState} from "react"
import {useLocation} from "react-router-dom"
import {styled} from "@mui/material/styles"
import {IconButton, adminTheme} from "@sequentech/ui-essentials"
import {TextField} from "@mui/material"
import {Menu, useSidebarState} from "react-admin"
import {TreeMenu} from "./election-events/TreeMenu"
import {faThLarge, faSearch, faPlusCircle} from "@fortawesome/free-solid-svg-icons"
import {cn} from "../../../lib/utils"
import {HorizontalBox} from "../../HorizontalBox"
import {Link} from "react-router-dom"

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

const activeRouteList = [
    "sequent_backend_election_event",
    "sequent_backend_election",
    "sequent_backend_contest",
    "sequent_backend_candidate",
]

export default function ElectionEvents() {
    const [open] = useSidebarState()
    const [searchInput, setSearchInput] = useState<string>("")

    function handleSearchChange(searchInput: string) {
        setSearchInput(searchInput)
    }

    const searchFilter = searchInput.trim()
        ? {
              "name@_like": searchInput.trim(),
          }
        : {}

    const location = useLocation()
    const isElectionEventActive = activeRouteList.some(
        (route) => location.pathname.search(route) > -1
    )

    const treeResourceNames = [
        "sequent_backend_election_event",
        "sequent_backend_election",
        "sequent_backend_contest",
        "sequent_backend_candidate",
    ]

    return (
        <>
            <div className={cn(isElectionEventActive && "bg-green-light")}>
                <HorizontalBox sx={{alignItems: "center"}}>
                    <MenuItem
                        to="/sequent_backend_election_event"
                        primaryText={open && "Election Events"}
                        leftIcon={<IconButton icon={faThLarge} fontSize="24px" />}
                        sx={{flexGrow: 2}}
                    />
                    <Link to="/sequent_backend_election_event/create">
                        <StyledIconButton icon={faPlusCircle} size="xs" />
                    </Link>
                </HorizontalBox>
                {open && isElectionEventActive && (
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
                        <TreeMenu
                            isOpen={open}
                            resourceNames={treeResourceNames}
                            filter={searchFilter}
                        />
                    </>
                )}
            </div>
        </>
    )
}
