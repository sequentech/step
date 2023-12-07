import React from "react"

import styled from "@emotion/styled"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"

import { diffLines } from 'diff';
import {Box, Accordion, AccordionDetails, AccordionSummary} from "@mui/material"

import Summary from "./election-publish.json"
import OldSummary from "./election-publish-old.json"

import { DiffView } from '@/components/DiffView';
import { EditElectionPublishActions } from './EditElectionPubllishActions';

const EditElectionPublishStyled = {
    Container: styled.div`
        display: flex;
        flex-direction: column;
        gap: 32px;
    `,
    AccordionHeaderTitle: styled.span`
        font-family: Roboto;
        font-size: 24px;
        font-weight: 700;
        line-height: 32px;
        letter-spacing: 0px;
        text-align: left;    
    `,
}

export const EditElectionPublish: React.FC = () => {
    const [diff, setDiff] = React.useState<any>('')
    const [expan, setExpan] = React.useState<string>('')
    const [oldJsonString, setOldJsonString] = React.useState<string>('')
    const [newJsonString, setNewJsonString] = React.useState<string>('')

    React.useEffect(() => {
        setNewJsonString(JSON.stringify(Summary, null, 2))
        setOldJsonString(JSON.stringify(OldSummary, null, 2))
    }, [])

    React.useEffect(() => {
        if (oldJsonString && newJsonString) {
            const diffText: any = diffLines(oldJsonString, newJsonString)
    
            console.log(diffText);
    
            setDiff(diffText)
        }
    }, [oldJsonString, newJsonString])

    if (!diff) {
        return <span>Loading ...</span>
    }

    return (
        <Box sx={{flexGrow: 2, flexShrink: 0}}>
            <EditElectionPublishActions />

            <EditElectionPublishStyled.Container>
                <Accordion sx={{width: "100%"}} expanded={expan == 'election-publish-diff'} onChange={() => setExpan('election-publish-diff')}>
                    <AccordionSummary expandIcon={<ExpandMoreIcon id="election-publish-diff" />}>
                        <EditElectionPublishStyled.AccordionHeaderTitle>
                            Change to be Publish
                        </EditElectionPublishStyled.AccordionHeaderTitle>
                    </AccordionSummary>
                    <AccordionDetails>
                        <DiffView diff={diff} />
                    </AccordionDetails>
                </Accordion>

                <Accordion sx={{width: "100%"}} expanded={expan === 'election-publish-history'} onChange={() => setExpan('election-publish-history')}>
                    <AccordionSummary expandIcon={<ExpandMoreIcon id="election-publish-history" />}>
                        <EditElectionPublishStyled.AccordionHeaderTitle>
                            Publish History
                        </EditElectionPublishStyled.AccordionHeaderTitle>
                    </AccordionSummary>
                    <AccordionDetails>
                        <span>Add correct resource</span>
                    </AccordionDetails>
                </Accordion>
            </EditElectionPublishStyled.Container>
        </Box>
    )
}
