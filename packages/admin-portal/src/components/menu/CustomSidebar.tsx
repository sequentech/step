// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {styled} from "@mui/material/styles"
import {ReactElement} from "react"
import {Drawer, DrawerProps, useScrollTrigger} from "@mui/material"
import lodashGet from "lodash/get"
import {useSidebarState} from "react-admin"

export const CustomSidebar = (props: SidebarProps) => {
    const {appBarAlwaysOn, children, closedSize, size, ...rest} = props
    const [open, setOpen] = useSidebarState()
    const trigger = useScrollTrigger()

    const toggleSidebar = () => setOpen(!open)

    return (
        <StyledDrawer
            variant="permanent"
            open={open}
            onClose={toggleSidebar}
            classes={SidebarClasses}
            className={trigger && !appBarAlwaysOn ? SidebarClasses.appBarCollapsed : ""}
            {...rest}
        >
            {children}
        </StyledDrawer>
    )
}

export interface SidebarProps extends DrawerProps {
    appBarAlwaysOn?: boolean
    children: ReactElement
    closedSize?: number
    size?: number
}

const PREFIX = "RaSidebar"

export const SidebarClasses = {
    docked: `${PREFIX}-docked`,
    paper: `${PREFIX}-paper`,
    paperAnchorLeft: `${PREFIX}-paperAnchorLeft`,
    paperAnchorRight: `${PREFIX}-paperAnchorRight`,
    paperAnchorTop: `${PREFIX}-paperAnchorTop`,
    paperAnchorBottom: `${PREFIX}-paperAnchorBottom`,
    paperAnchorDockedLeft: `${PREFIX}-paperAnchorDockedLeft`,
    paperAnchorDockedTop: `${PREFIX}-paperAnchorDockedTop`,
    paperAnchorDockedRight: `${PREFIX}-paperAnchorDockedRight`,
    paperAnchorDockedBottom: `${PREFIX}-paperAnchorDockedBottom`,
    modal: `${PREFIX}-modal`,
    fixed: `${PREFIX}-fixed`,
    appBarCollapsed: `${PREFIX}-appBarCollapsed`,
}

export const StyledDrawer = styled(Drawer, {
    name: PREFIX,
    slot: "Root",
    overridesResolver: (props, styles) => styles.root,
    shouldForwardProp: () => true,
})(({open, theme}) => ({
    height: "calc(100vh - 3em)",
    marginTop: 0,
    transition: theme.transitions.create("margin", {
        easing: theme.transitions.easing.easeOut,
        duration: theme.transitions.duration.enteringScreen,
    }),
    [`&.${SidebarClasses.appBarCollapsed}`]: {
        // compensate the margin of the Layout appFrame instead of removing it in the Layout
        // because otherwise, the appFrame content without margin may revert the scrollTrigger,
        // leading to a visual jiggle
        marginTop: theme.spacing(-6),
        [theme.breakpoints.down("sm")]: {
            marginTop: theme.spacing(-7),
        },
        transition: theme.transitions.create("margin", {
            easing: theme.transitions.easing.sharp,
            duration: theme.transitions.duration.leavingScreen,
        }),
    },
    [`& .${SidebarClasses.docked}`]: {},
    [`& .${SidebarClasses.paper}`]: {},
    [`& .${SidebarClasses.paperAnchorLeft}`]: {},
    [`& .${SidebarClasses.paperAnchorRight}`]: {},
    [`& .${SidebarClasses.paperAnchorTop}`]: {},
    [`& .${SidebarClasses.paperAnchorBottom}`]: {},
    [`& .${SidebarClasses.paperAnchorDockedLeft}`]: {},
    [`& .${SidebarClasses.paperAnchorDockedTop}`]: {},
    [`& .${SidebarClasses.paperAnchorDockedRight}`]: {},
    [`& .${SidebarClasses.paperAnchorDockedBottom}`]: {},
    [`& .${SidebarClasses.modal}`]: {},

    [`& .${SidebarClasses.fixed}`]: {
        "position": "fixed",
        "overflowX": "hidden",
        // hide scrollbar
        "scrollbarWidth": "none",
        "msOverflowStyle": "none",
        "&::-webkit-scrollbar": {
            display: "none",
        },
    },

    [`& .MuiPaper-root`]: {
        position: "relative",
        width: open
            ? lodashGet(theme, "sidebar.width", DRAWER_WIDTH)
            : lodashGet(theme, "sidebar.closedWidth", CLOSED_DRAWER_WIDTH),
        transition: theme.transitions.create("width", {
            easing: theme.transitions.easing.sharp,
            duration: theme.transitions.duration.leavingScreen,
        }),
        backgroundColor: "transparent",
        borderRight: "none",
        [theme.breakpoints.only("xs")]: {
            marginTop: 0,
            position: "inherit",
            backgroundColor: theme.palette.background.default,
        },
        [theme.breakpoints.up("md")]: {
            border: "none",
        },
        overflowX: "hidden",
        zIndex: "inherit",
    },
}))

export const DRAWER_WIDTH = 240
export const CLOSED_DRAWER_WIDTH = 55
