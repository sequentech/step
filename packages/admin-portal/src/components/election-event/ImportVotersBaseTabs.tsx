import React from "react"
import {ShowBase} from "react-admin"
import {ImportVotersTabs} from "./ImportVotersTabs"
import { Box } from '@mui/material'

export const ImportVotersBaseTabs: React.FC = () => {
    return (
        <Box sx={{padding: "16px"}}>
            <ImportVotersTabs />
        </Box>
    )
}
