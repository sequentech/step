import React from "react"

import ExpandMoreIcon from "@mui/icons-material/ExpandMore"

import {diffLines} from "diff"
import { Diff } from 'react-diff-view';
import {Box, Accordion, AccordionDetails, AccordionSummary} from "@mui/material"

import Summary from "./election-publish.json"
import OldSummary from "./election-publish-old.json"

export const EditElectionPublish: React.FC = () => {
    const [diff, setDiff] = React.useState("")
    const [expan, setExpan] = React.useState(false)
    const [oldJsonString, setOldJsonString] = React.useState("")
    const [newJsonString, setNewJsonString] = React.useState("")

    React.useEffect(() => {
        setNewJsonString(JSON.stringify(Summary, null, 2))
        setOldJsonString(JSON.stringify(OldSummary, null, 2))

        setDiff(diffLines(oldJsonString, newJsonString))
    }, [])

    return (
        <Box sx={{flexGrow: 2, flexShrink: 0}}>
            <Accordion sx={{width: "100%"}} expanded={expan} onChange={() => setExpan(!expan)}>
                <AccordionSummary expandIcon={<ExpandMoreIcon id="election-data-general" />}>
                    <h1>Change to be Publish</h1>
                </AccordionSummary>
                <AccordionDetails>
                    <Diff diffType="modify" />
                </AccordionDetails>
            </Accordion>
        </Box>
    )
}
