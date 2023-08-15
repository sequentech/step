// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Menu} from "react-admin"
import {faThLarge, faUsers, faCog, faStar} from "@fortawesome/free-solid-svg-icons"
import {IconButton} from "@sequentech/ui-essentials"

export const CustomMenu = () => (
    <Menu>
        <Menu.Item
            to="/sequent_backend_election_event"
            primaryText="Election Events"
            leftIcon={<IconButton icon={faThLarge} fontSize="14px" />}
        />
        <Menu.Item
            to="/user-roles"
            primaryText="User and Roles"
            leftIcon={<IconButton icon={faUsers} fontSize="14px" />}
        />
        <Menu.Item
            to="/settings"
            primaryText="Settings"
            leftIcon={<IconButton icon={faCog} fontSize="14px" />}
        />
        <Menu.Item
            to="/messages"
            primaryText="Messages"
            leftIcon={<IconButton icon={faStar} fontSize="14px" />}
        />
    </Menu>
)
