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
    className?: string
}

export const VoterInformationLetterDocumentAccess = ({
    taskId,
    pdfPassword,
    loading = false,
    onReveal,
    className,
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
            className={["voter-information-letter-document-access", className]
                .filter(Boolean)
                .join(" ")}
            sx={{width: "100%"}}
            expanded={expanded}
            onChange={(_, nextExpanded) => setExpanded(nextExpanded)}
        >
            <AccordionSummary
                className="voter-information-letter-document-access-summary"
                expandIcon={
                    <ExpandMoreIcon className="voter-information-letter-document-access-expand-icon" />
                }
            >
                <WizardStyles.AccordionTitle className="voter-information-letter-document-access-title">
                    {t("tasksScreen.documentAccess.title")}
                </WizardStyles.AccordionTitle>
            </AccordionSummary>
            <WizardStyles.AccordionDetails className="voter-information-letter-document-access-details">
                <VoterInformationLetterPasswordAccess
                    className="voter-information-letter-document-access-password"
                    pdfPassword={pdfPassword}
                    loading={loading}
                    onReveal={onReveal}
                />
            </WizardStyles.AccordionDetails>
        </Accordion>
    )
}
