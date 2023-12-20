import React from "react"
import {ShowBase} from "react-admin"
import {ImportVotersTabs} from "./ImportVotersTabs"
import {Box} from "@mui/material"

interface ImportVotersBaseProps {
    doRefresh: () => void
}

export const ImportVotersBaseTabs: React.FC<ImportVotersBaseProps> = (props) => {
    return (
        <Box sx={{padding: "16px"}}>
            <ImportVotersTabs doRefresh={props.doRefresh} />
        </Box>
    )
}
