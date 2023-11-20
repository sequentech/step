import styled from "@emotion/styled"
import {IconButton} from "@mui/material"
import {adminTheme} from "@sequentech/ui-essentials"
import React from "react"
import {Identifier, useRecordContext} from "react-admin"


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
}

interface ActionsColumnProps {
    actions: Array<{icon: React.ReactNode; action: (id: Identifier) => void}>
}

export const ActionsColumn: React.FC<ActionsColumnProps> = (props) => {
    const record = useRecordContext()
    const {actions} = props

    const StyledIconButton = styled(IconButton)`
        color: ${adminTheme.palette.brandColor};
        font-size: 18px;
        margin-left: 8px;
    `

    return (
        <>
            {actions && actions.length > 0
                ? actions.map((action, index) => (
                      <StyledIconButton key={index} onClick={() => action.action(record.id)}>
                          {action.icon}
                      </StyledIconButton>
                  ))
                : null}
        </>
    )
}
