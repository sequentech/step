// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useState} from "react"
import {useLocation} from "react-router-dom"
import {styled} from "@mui/material/styles"
import {IconButton, adminTheme} from "@sequentech/ui-essentials"
import {TextField} from "@mui/material"
import {Menu, useSidebarState} from "react-admin"
import {TreeMenu} from "../../TreeMenu"
import {faThLarge, faSearch} from "@fortawesome/free-solid-svg-icons"
import {cn} from "../../../lib/utils"

const MenuItem = styled(Menu.Item)`
    color: ${adminTheme.palette.brandColor};

    &.RaMenuItemLink-active,
    .MuiIconButton-root {
        color: ${adminTheme.palette.brandColor};
    }
`

const activeRouteList = [
    "sequent_backend_election_event",
    "sequent_backend_election",
    "sequent_backend_contest",
    "sequent_backend_candidate",
]

export default function ElectionEvents() {
    const [open, setOpen] = useSidebarState()
    const [search, setSearch] = useState<string | null>(null)

    const handleSearchChange = (event: React.ChangeEvent<HTMLInputElement>) => {
        // Update the state with the input field's current value
        setSearch(event.target.value)
    }

    const location = useLocation()
    const isElectionEventActive = activeRouteList.some(
        (route) => location.pathname.search(route) > -1
    )

    return (
        <>
            <div className={cn(isElectionEventActive && "bg-green-light")}>
                <MenuItem
                    to="/sequent_backend_election_event"
                    primaryText={open && "Election Events"}
                    leftIcon={<IconButton icon={faThLarge} fontSize="24px" />}
                />
                {open && isElectionEventActive && (
                    <>
                        <div className="flex bg-white px-4">
                            <TextField
                                label="Search"
                                size="small"
                                value={search}
                                onChange={handleSearchChange}
                            />
                            <IconButton icon={faSearch} fontSize="18px" sx={{margin: "0 12px"}} />
                        </div>
                        <TreeMenu isOpen={open} />
                    </>
                )}
            </div>
        </>
    )
}
