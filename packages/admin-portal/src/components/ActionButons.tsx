// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import styled from "@emotion/styled"
import {IconButton} from "@mui/material"
import {adminTheme} from "@sequentech/ui-essentials"
import React from "react"
import {Identifier, RaRecord, useRecordContext} from "react-admin"

/*  
        In the component where you want to use the actions column:
        
        - define the functions and the actions custom column to be showned
        - define teh actions array with the actions to be showned
        - add the ActionsColumn as a column to the list as the final column as a normal one

        Format: {icon: <Icon />, action: (id: Identifier) => void}
    */

export interface Action {
    icon: React.ReactNode
    action: (id: Identifier) => void
    showAction?: (id: Identifier) => boolean
    label?: string
    className?: string
    saveRecordAction?: (record: RaRecord<Identifier>) => void
    key?: string
}

interface ActionsColumnProps {
    label?: string
    actions: Array<Action>
}

export const StyledIconButton = styled(IconButton)`
    color: ${adminTheme.palette.brandColor};
    font-size: 18px;
    margin-left: 8px;
`

export const ActionsColumn: React.FC<ActionsColumnProps> = (props) => {
    const record = useRecordContext()
    const {actions} = props

    const filteredActions = actions.filter(
        (action) => !action.showAction || action.showAction(record.id)
    )

    return (
        <>
            {filteredActions && filteredActions.length > 0
                ? filteredActions.map((action, index) => (
                      <StyledIconButton
                          className={action.className ?? ""}
                          key={index}
                          onClick={() => action.action(record.id)}
                      >
                          {action.icon}
                      </StyledIconButton>
                  ))
                : null}
        </>
    )
}
