import React from "react"

import styled from "@emotion/styled"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"

import {Box, Accordion, AccordionDetails, AccordionSummary, CircularProgress} from "@mui/material"

import Summary from "./election-publish.json"
import OldSummary from "./election-publish-old.json"

import {DiffView} from "@/components/DiffView"
import {PublishActions} from "./PublishActions"
import {useTranslation} from "react-i18next"

const PublishStyled = {
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
    Loading: styled.div`
        display: flex;
        height: 60vh;
        justify-content: center;
        align-items: center;
    `,
}

export const Publish: React.FC = () => {
    const {t} = useTranslation()
    const [expan, setExpan] = React.useState<string>("election-publish-diff")

    return (
        <Box sx={{flexGrow: 2, flexShrink: 0}}>
            <PublishActions />

            <PublishStyled.Container>
                <Accordion
                    sx={{width: "100%"}}
                    expanded={expan == "election-publish-diff"}
                    onChange={() => setExpan("election-publish-diff")}
                >
                    <AccordionSummary expandIcon={<ExpandMoreIcon id="election-publish-diff" />}>
                        <PublishStyled.AccordionHeaderTitle>
                            {t("publish.header.change")}
                        </PublishStyled.AccordionHeaderTitle>
                    </AccordionSummary>
                    <AccordionDetails>
                        <DiffView
                            currentTitle={t("publish.label.current")}
                            diffTitle={t("publish.label.diff")}
                            current={OldSummary}
                            modify={Summary}
                        />
                    </AccordionDetails>
                </Accordion>

                <Accordion
                    sx={{width: "100%"}}
                    expanded={expan === "election-publish-history"}
                    onChange={() => setExpan("election-publish-history")}
                >
                    <AccordionSummary expandIcon={<ExpandMoreIcon id="election-publish-history" />}>
                        <PublishStyled.AccordionHeaderTitle>
                            {t("publish.header.history")}
                        </PublishStyled.AccordionHeaderTitle>
                    </AccordionSummary>
                    <AccordionDetails>
                        <span>Add correct resource</span>
                    </AccordionDetails>
                </Accordion>
            </PublishStyled.Container>
        </Box>
    )
}
