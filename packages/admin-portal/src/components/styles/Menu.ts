//SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {css} from "@emotion/react"
import styled from "@emotion/styled"
import AddIcon from "@mui/icons-material/Add"
import {NavLink} from "react-router-dom"
import HowToVoteIcon from "@mui/icons-material/HowToVote"
import AddCircleIcon from "@mui/icons-material/AddCircle"
import Box from "@mui/material/Box"
import {adminTheme} from "@sequentech/ui-essentials"
import {Typography} from "@mui/material"

export const divContainer = css`
    flex: 0 0 auto;
    width: 1.5rem;
    height: 1.5rem;
`

export const MenuStyles = {
    SideMenuContainer: styled("ul")`
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
    `,

    SideMenuActiveItem: styled("li")<{isArchivedElectionEvents: boolean}>`
        padding-left: 1rem;
        padding-right: 1rem;
        padding-top: 0.5rem;
        padding-bottom: 0.5rem;
        cursor: pointer;
        color: ${({isArchivedElectionEvents}) =>
            !isArchivedElectionEvents
                ? adminTheme.palette.brandColor
                : adminTheme.palette.secondary.main};
        border-bottom: ${({isArchivedElectionEvents}) =>
            !isArchivedElectionEvents ? `2px solid ${adminTheme.palette.brandSuccess}` : "none"};
    `,

    SideMenuArchiveItem: styled("li")<{isArchivedElectionEvents: boolean}>`
        padding-left: 1rem;
        padding-right: 1rem;
        padding-top: 0.5rem;
        padding-bottom: 0.5rem;
        cursor: pointer;
        color: ${({isArchivedElectionEvents}) =>
            isArchivedElectionEvents
                ? adminTheme.palette.brandColor
                : adminTheme.palette.secondary.main};
        border-bottom: ${({isArchivedElectionEvents}) =>
            isArchivedElectionEvents ? `2px solid ${adminTheme.palette.brandSuccess}` : "none"};
    `,

    RefreshAction: styled("li")`
        padding-top: 0.5rem;
        cursor: pointer;
        width: 100%;
        display: inline-block;
        float: right;
        text-align: right;

        &:hover {
            opacity: 0.5;
        }
        &:active {
            opacity: 1;
        }
    `,

    EmptyStateContainer: styled("div")`
        padding: 1rem;
        background-color: white;
    `,

    TreeLeavesContainer: styled("div")`
        display: flex;
        flex-direction: column;
        margin-left: 0.75rem;
    `,

    CreateElectionContainer: styled("div")`
        display: flex;
        align-items: center;
        color: ${adminTheme.palette.secondary.main};
        & > *:not(:last-child) {
            margin-right: 0.5rem;
        }
    `,

    StyledAddIcon: styled(AddIcon)`
        flex: 0 0 auto;
    `,

    StyledNavLink: styled(NavLink)`
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
            border-bottom-color: ${adminTheme.palette.secondary.main};
        }
    `,
    StyledNavLinkButton: styled(Typography)`
        flex-grow: 1;
        padding-top: 0.375rem;
        padding-bottom: 0.375rem;
        border-bottom-width: 2px;
        border-bottom-color: white;
        cursor: pointer;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        background-color: transparent !important;
        background: transparent !important;
        color: ${adminTheme.palette.secondary.main} !important;
        margin: 0px;

        &:hover {
            border-bottom-color: ${adminTheme.palette.secondary.main};
        }
    `,
    StyledHiddenDiv: styled("div")`
        ${divContainer}
        visibility: hidden;
    `,

    ItemContainer: styled("p")`
        display: flex;
        align-items: center;
        & > *:not(:last-child) {
            margin-right: 0.5rem;
        }
    `,
    HowToVoteStyledIcon: styled(HowToVoteIcon)`
        color: ${adminTheme.palette.brandColor};
    `,
    TreeMenuIconContaier: styled.div`
        ${divContainer}
        cursor: pointer;
        color: black;
    `,
    StyledSideBarNavLink: styled(NavLink)<{multiline?: string | undefined}>`
        flex-grow: 1;
        padding-top: 0.375rem;
        padding-bottom: 0.375rem;
        color: black;
        border-bottom: 2px solid white;
        cursor: pointer;
        ${(data) =>
            data.multiline && data.multiline === "true"
                ? `
            /* Allow up to two lines of text */
            display: -webkit-box;
            -webkit-line-clamp: 2;
            -webkit-box-orient: vertical;
            white-space: normal;
            overflow: hidden;
            `
                : "white-space: nowrap;"}
        overflow: hidden;
        text-overflow: ellipsis;

        &.active {
            border-bottom-color: ${adminTheme.palette.brandColor};
        }
    `,

    MenuActionContainer: styled("div")`
        visibility: hidden;
    `,
    StyledIconContainer: styled("p")`
        ${divContainer}
        cursor: pointer
    `,

    StyledAddCircleIcon: styled(AddCircleIcon)`
        color: ${adminTheme.palette.brandColor};
    `,
    StyledDiv: styled(Box)<{isWidth: boolean}>`
        flex: 0 0 auto;
        height: 1.5rem;
        width: ${({isWidth}) => (isWidth ? "1.5rem" : "auto")};
    `,
}

const highlightedItem = css`
    #StyledSideBarNavLink {
        border-bottom-color: ${adminTheme.palette.brandColor};
    }
    #MoreHorizIcon {
        visibility: visible;
    }
`

export const TreeMenuItemContainer = styled.div<{isClicked: boolean}>`
    display: flex;
    text-align: left;
    align-items: center;

    & > *:not(:last-child) {
        margin-right: 0.5rem;
    }

    &:hover {
        ${highlightedItem}
    }

    ${({isClicked}) => isClicked && highlightedItem}
`
