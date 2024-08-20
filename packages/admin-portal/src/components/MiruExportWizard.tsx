// SPDX-FileCopyrightText: 2024 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Accordion, AccordionSummary, Box, CircularProgress} from "@mui/material"
import React, {useMemo} from "react"
import {WizardStyles} from "./styles/WizardStyles"
import {TallyStyles} from "./styles/TallyStyles"
import {MiruServers} from "./MiruServers"
import {ExportButton} from "./MiruExport"
import {MiruSignatures} from "./MiruSignatures"
import {theme, DropFile} from "@sequentech/ui-essentials"
import {Logs} from "./Logs"
import {MiruPackageDownload} from "./MiruPackageDownload"
import {IExpanded} from "@/resources/Tally/TallyCeremony"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"
import {
    Sequent_Backend_Area,
    Sequent_Backend_Results_Event,
    Sequent_Backend_Tally_Session_Execution
} from "@/gql/graphql"
import {IMiruTransmissionPackageData} from "@/types/miru"
import {IResultDocuments} from "@/types/results"
import {useTranslation} from "react-i18next"

interface IMiruExportWizardProps {
    tallySessionExecution?: Sequent_Backend_Tally_Session_Execution
    expandedExports: IExpanded
    resultsEvent: Sequent_Backend_Results_Event[] | undefined
    setExpandedDataExports: React.Dispatch<React.SetStateAction<IExpanded>>
    transmissionLoading: boolean
    documents: IResultDocuments | null
    handleSendTransmissionPackage: () => void
    selectedTallySessionData: IMiruTransmissionPackageData | null
    uploading: boolean
    isTrustee: boolean
    area: Sequent_Backend_Area | null
    errors: String | null
    handleUploadSignature: (files: FileList | null) => Promise<void>
}

export const MiruExportWizard: React.FC<IMiruExportWizardProps> = ({
    tallySessionExecution,
    expandedExports,
    resultsEvent,
    setExpandedDataExports,
    transmissionLoading,
    handleSendTransmissionPackage,
    selectedTallySessionData,
    uploading,
    documents,
    errors,
    area,
    isTrustee,
    handleUploadSignature,
}) => {
    const {t, i18n} = useTranslation()

    const signaturesStatusColor: () => string = () => {
        let signed = signedCount()
        let trustees = trusteeCount()
        
        return (signed < trustees) ? theme.palette.info.main : theme.palette.brandSuccess
    }

    const signedCount: () => number = () => {
        let signatures = selectedTallySessionData?.documents[
            selectedTallySessionData?.documents.length - 1
        ].signatures ?? []

        return signatures
            .filter(signature => !signature.signature || !signature.pub_key).length
    }

    const trusteeCount: () => number = () => {
        let trustees = tallySessionExecution?.status?.trustees ?? []
        return trustees.length
    }

    return (
        <>
            {isTrustee && (
                <Accordion
                    sx={{width: "100%"}}
                    expanded={expandedExports["tally-miru-upload"]}
                    onChange={() =>
                        setExpandedDataExports((prev: IExpanded) => ({
                            ...prev,
                            "tally-miru-upload": !prev["tally-miru-upload"],
                        }))
                    }
                >
                    <AccordionSummary>
                        <Box className="flex flex-col items-start">
                            <WizardStyles.AccordionTitle>
                                {t("tally.uploadTransmissionPackage")}
                            </WizardStyles.AccordionTitle>
                            <WizardStyles.AccordionSubTitle>
                                {t("tally.uploadTransmissionPackageDesc")}
                            </WizardStyles.AccordionSubTitle>
                        </Box>
                    </AccordionSummary>
                    <WizardStyles.AccordionDetails style={{zIndex: 100}}>
                        <DropFile handleFiles={handleUploadSignature} />
                        <WizardStyles.StatusBox>
                            {uploading ? <WizardStyles.DownloadProgress /> : null}
                            {errors ? (
                                <WizardStyles.ErrorMessage variant="body2">
                                    {errors}
                                </WizardStyles.ErrorMessage>
                            ) : null}
                        </WizardStyles.StatusBox>
                    </WizardStyles.AccordionDetails>
                </Accordion>
            )}
            <Accordion
                sx={{width: "100%"}}
                expanded={expandedExports["tally-miru-signatures"]}
                onChange={() =>
                    setExpandedDataExports((prev: IExpanded) => ({
                        ...prev,
                        "tally-miru-signatures": !prev["tally-miru-signatures"],
                    }))
                }
            >
                <AccordionSummary expandIcon={<ExpandMoreIcon id="tally-miru-signatures" />}>
                    <WizardStyles.AccordionTitle>
                        {t("tally.transmissionPackage.signatures.title")}
                    </WizardStyles.AccordionTitle>
                    <WizardStyles.CeremonyStatus
                        sx={{
                            backgroundColor: signaturesStatusColor(),
                            color: theme.palette.background.default,
                            textTransform: "uppercase"
                        }}
                        label={t("tally.transmissionPackage.signatures.status", {
                            signed: signedCount(),
                            total: trusteeCount(),
                        })}
                    />
                </AccordionSummary>
                <WizardStyles.AccordionDetails style={{zIndex: 100}}>
                    <WizardStyles.AccordionSubTitle>
                        {t("tally.transmissionPackage.signatures.description")}
                    </WizardStyles.AccordionSubTitle>
                    <MiruSignatures
                        signatures={
                            selectedTallySessionData?.documents[
                                selectedTallySessionData?.documents.length - 1
                            ].signatures ?? []
                        }
                        tallySessionExecution={tallySessionExecution}
                    />
                </WizardStyles.AccordionDetails>
            </Accordion>
            <Accordion
                sx={{width: "100%"}}
                expanded={expandedExports["tally-miru-servers"]}
                onChange={() =>
                    setExpandedDataExports((prev: IExpanded) => ({
                        ...prev,
                        "tally-miru-servers": !prev["tally-miru-servers"],
                    }))
                }
            >
                <AccordionSummary expandIcon={<ExpandMoreIcon id="tally-miru-servers" />}>
                    <WizardStyles.AccordionTitle>
                        {t("tally.transmissionPackage.destinationServers.title")}
                    </WizardStyles.AccordionTitle>
                </AccordionSummary>
                <WizardStyles.AccordionDetails style={{zIndex: 100}}>
                    <MiruServers servers={selectedTallySessionData?.servers ?? []} />
                </WizardStyles.AccordionDetails>
            </Accordion>

            <Logs logs={selectedTallySessionData?.logs} />
        </>
    )
}
