// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useState} from "react"
import {
    Accordion,
    AccordionSummary,
    Alert,
    Button,
    IconButton,
    InputAdornment,
    Stack,
    TextField,
    Typography,
} from "@mui/material"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"
import ContentCopyIcon from "@mui/icons-material/ContentCopy"
import VisibilityIcon from "@mui/icons-material/Visibility"
import DownloadIcon from "@mui/icons-material/Download"
import {useLazyQuery} from "@apollo/client"
import {useNotify} from "react-admin"
import {useTranslation} from "react-i18next"
import {GetVoterInformationLetterPasswordQuery} from "@/gql/graphql"
import {GET_VOTER_INFORMATION_LETTER_PASSWORD} from "@/queries/VoterInformationLetter"
import {WizardStyles} from "@/components/styles/WizardStyles"
import {IPermissions} from "@/types/keycloak"
import {DownloadDocument} from "@/resources/User/DownloadDocument"
import {DownloaButton} from "@/components/styles/WidgetStyle"

interface VoterInformationLetterDocumentAccessProps {
    taskId: string
    documentId: string
    electionEventId: string
}

export const VoterInformationLetterDocumentAccess = ({
    taskId,
    documentId,
    electionEventId,
}: VoterInformationLetterDocumentAccessProps) => {
    const {t} = useTranslation()
    const notify = useNotify()
    const [expanded, setExpanded] = useState(false)
    const [pdfPassword, setPdfPassword] = useState<string>()
    const [downloading, setDownloading] = useState(false)
    const [getPassword, {loading}] = useLazyQuery<GetVoterInformationLetterPasswordQuery>(
        GET_VOTER_INFORMATION_LETTER_PASSWORD,
        {
            fetchPolicy: "no-cache",
            context: {
                headers: {
                    "x-hasura-role": IPermissions.VOTER_INFORMATION_LETTER,
                },
            },
        }
    )

    useEffect(() => {
        setPdfPassword(undefined)
        setExpanded(false)
        setDownloading(false)
    }, [taskId])

    const revealPassword = async () => {
        try {
            const {data} = await getPassword({variables: {taskId}})
            const password = data?.get_voter_information_letter_password?.pdf_password
            if (!password) {
                throw new Error("PDF password was not returned")
            }
            setPdfPassword(password)
        } catch {
            notify(t("tasksScreen.documentAccess.passwordError"), {type: "error"})
        }
    }

    const copyPassword = async () => {
        if (!pdfPassword) {
            return
        }
        try {
            await navigator.clipboard.writeText(pdfPassword)
            notify(t("tasksScreen.documentAccess.passwordCopied"), {type: "success"})
        } catch {
            notify(t("tasksScreen.documentAccess.copyError"), {type: "error"})
        }
    }

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
                <Stack spacing={2}>
                    <Alert severity="warning">
                        {t("tasksScreen.documentAccess.sensitivityNotice")}
                    </Alert>
                    {pdfPassword ? (
                        <TextField
                            fullWidth
                            label={t("tasksScreen.documentAccess.passwordLabel")}
                            value={pdfPassword}
                            slotProps={{
                                input: {
                                    readOnly: true,
                                    endAdornment: (
                                        <InputAdornment position="end">
                                            <IconButton
                                                onClick={() => void copyPassword()}
                                                edge="end"
                                                aria-label={t(
                                                    "tasksScreen.documentAccess.copyPassword"
                                                )}
                                            >
                                                <ContentCopyIcon />
                                            </IconButton>
                                        </InputAdornment>
                                    ),
                                },
                            }}
                        />
                    ) : (
                        <Button
                            variant="outlined"
                            startIcon={<VisibilityIcon />}
                            onClick={() => void revealPassword()}
                            disabled={loading}
                        >
                            {t("tasksScreen.documentAccess.showPassword")}
                        </Button>
                    )}
                    <Typography variant="body2">
                        {t("tasksScreen.documentAccess.guidance")}
                    </Typography>
                    <DownloaButton
                        onClick={() => setDownloading(true)}
                        disabled={downloading}
                        label={String(t("tasksScreen.widget.downloadDocument"))}
                    >
                        <DownloadIcon />
                    </DownloaButton>
                    {downloading ? (
                        <DownloadDocument
                            documentId={documentId}
                            electionEventId={electionEventId}
                            fileName={null}
                            onDownload={() => setDownloading(false)}
                        />
                    ) : null}
                </Stack>
            </WizardStyles.AccordionDetails>
        </Accordion>
    )
}
