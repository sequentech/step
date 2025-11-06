// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {IconButton, Menu, MenuItem} from "@mui/material"
import {GridMoreVertIcon} from "@mui/x-data-grid"
import React from "react"
import {useRecordContext} from "react-admin"
import {Action} from "./ActionButons"
import {makeStyles} from "@mui/styles"

/*  
        In the component where you want to use the actions column as popover menu:
        
        - define the functions and the actions custom column to be showned
        - define teh actions array with the actions to be showned
        - add the ActionsColumn as a column to the list as the final column as a normal one

        Format: {icon: <Icon />, action: (id: Identifier) => void, label: string}
    */

interface ListActionsMenuProps {
    actions: Array<Action>
}

const useStyles = makeStyles({
    menu: {
        width: "max-content",
    },
})

export const ListActionsMenu: React.FC<ListActionsMenuProps> = (props) => {
    const record = useRecordContext()
    const {actions} = props
    const [anchorEl, setAnchorEl] = React.useState<null | HTMLElement>(null)
    const open = Boolean(anchorEl)
    const classes = useStyles()
    const handleClick = (event: React.MouseEvent<HTMLElement>) => {
        setAnchorEl(event.currentTarget)
    }
    const handleClose = () => {
        setAnchorEl(null)
    }

    const filteredActions = actions.filter(
        (action) => !action.showAction || action.showAction(record.id)
    )

    const handleClickAction = (action: Action) => {
        action.action(record.id)
        if (action.saveRecordAction) {
            action.saveRecordAction(record)
        }
        handleClose()
    }

    return (
        <div>
            <IconButton
                id="actions-menu-button"
                aria-controls={open ? "actions-menu" : undefined}
                aria-haspopup="true"
                aria-expanded={open ? "true" : undefined}
                onClick={handleClick}
            >
                <GridMoreVertIcon />
            </IconButton>
            <Menu
                classes={{paper: classes.menu}}
                id="actions-menu"
                aria-labelledby="actions-menu-button"
                anchorEl={anchorEl}
                open={open}
                onClose={handleClose}
                anchorOrigin={{
                    vertical: "top",
                    horizontal: "left",
                }}
                transformOrigin={{
                    vertical: "top",
                    horizontal: "left",
                }}
            >
                {filteredActions && filteredActions.length > 0
                    ? filteredActions.map((action, index) => (
                          <MenuItem
                              key={index}
                              onClick={() => handleClickAction(action)}
                              sx={{display: "flex", gap: "8px"}}
                              className={action.className ?? ""}
                          >
                              {action.icon}
                              {action.label || ""}
                          </MenuItem>
                      ))
                    : null}
            </Menu>
        </div>
    )
}
