import React from "react"
import {ShowBase} from "react-admin"
import { ContestTabs } from './ContestTabs'

export const ContestBaseTabs: React.FC = () => {
    return (
        <ShowBase>
            <ContestTabs />
        </ShowBase>
    )
}
