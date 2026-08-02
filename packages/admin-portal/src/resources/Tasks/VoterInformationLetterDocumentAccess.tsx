// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useState} from "react"
import {Accordion, AccordionSummary} from "@mui/material"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"
import {useTranslation} from "react-i18next"
import {WizardStyles} from "@/components/styles/WizardStyles"
import {VoterInformationLetterPasswordAccess} from "./VoterInformationLetterPasswordAccess"

interface VoterInformationLetterDocumentAccessProps {
    taskId: string
    pdfPassword?: string
    loading?: boolean
    onReveal: () => void
}

export const VoterInformationLetterDocumentAccess = ({
    taskId,
    pdfPassword,
    loading = false,
    onReveal,
}: VoterInformationLetterDocumentAccessProps) => {
    const {t} = useTranslation()
    const [expanded, setExpanded] = useState(true)

    useEffect(() => {
        setExpanded(true)
    }, [taskId])

    useEffect(() => {
        if (pdfPassword) {
            setExpanded(true)
        }
    }, [pdfPassword])

    return (
        <Accordion
            sx={{width: "100%"}}
            expanded={expanded}
            onChange={(_, nextExpanded) => setExpanded(nextExpanded)}
        >
            <AccordionSummary expandIcon={<ExpandMoreIcon />}>
                <WizardStyles.AccordionTitle>
                    {t("tasksScreen.documentAccess.title")}
                </WizardStyles.AccordionTitle>
            </AccordionSummary>
            <WizardStyles.AccordionDetails>
                <VoterInformationLetterPasswordAccess
                    pdfPassword={pdfPassword}
                    loading={loading}
                    onReveal={onReveal}
                />
            </WizardStyles.AccordionDetails>
        </Accordion>
    )
}
