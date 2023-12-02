import React from "react"
import {ShowBase} from "react-admin"
import {ElectionTabs} from "../Election/ElectionTabs"

export const ElectionBaseTabs: React.FC = () => {
    return (
        <ShowBase>
            <ElectionTabs />
        </ShowBase>
    )
}
