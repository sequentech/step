import {Accordion, AccordionSummary, CircularProgress} from "@mui/material"
import React from "react"
import {WizardStyles} from "./styles/WizardStyles"
import {TallyStyles} from "./styles/TallyStyles"
import {MiruServers} from "./MiruServers"
import {ExportButton} from "./MiruExport"
import {MiruSignatures} from "./MiruSignatures"
import {DropFile} from "@sequentech/ui-essentials"
import {Logs} from "./Logs"
import {MiruPackageDownload} from "./MiruPackageDownload"
import {IExpanded} from "@/resources/Tally/TallyCeremony"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"
import {Sequent_Backend_Results_Event} from "@/gql/graphql"
import {IMiruTransmissionPackageData} from "@/types/miru"
import {IResultDocuments} from "@/types/results"
import {useTranslation} from "react-i18next"

interface IMiruExportWizardProps {
    expandedExports: IExpanded
    resultsEvent: Sequent_Backend_Results_Event[] | undefined
    setExpandedDataExports: React.Dispatch<React.SetStateAction<IExpanded>>
    transmissionLoading: boolean
    documents: IResultDocuments | null
    handleSendTransmissionPackage: () => void
    selectedTallySessionData: IMiruTransmissionPackageData | null
    uploading: boolean
    errors: String | null
    handleUploadSignature: (files: FileList | null) => Promise<void>
}

export const MiruExportWizard = ({
    expandedExports,
    resultsEvent,
    setExpandedDataExports,
    transmissionLoading,
    handleSendTransmissionPackage,
    selectedTallySessionData,
    uploading,
    documents,
    errors,
    handleUploadSignature,
}: IMiruExportWizardProps) => {
    const {t, i18n} = useTranslation()

    return (
        <>
            <Accordion
                sx={{width: "100%"}}
                expanded={expandedExports["tally-download-package"]}
                onChange={() =>
                    setExpandedDataExports((prev: IExpanded) => ({
                        ...prev,
                        "tally-download-package": !prev["tally-download-package"],
                    }))
                }
            >
                <AccordionSummary>
                    <WizardStyles.AccordionTitle>
                        {t("tally.downloadTransmissionPackage")}
                    </WizardStyles.AccordionTitle>
                    <TallyStyles.StyledSpacing>
                        {resultsEvent?.[0] && documents ? (
                            <MiruPackageDownload
                                documents={selectedTallySessionData?.documents ?? []}
                                electionEventId={resultsEvent?.[0].election_event_id}
                            />
                        ) : null}
                    </TallyStyles.StyledSpacing>
                </AccordionSummary>
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
                        {t("tally.TransmissionPackageServers")}
                    </WizardStyles.AccordionTitle>
                </AccordionSummary>
                <WizardStyles.AccordionDetails style={{zIndex: 100}}>
                    <MiruServers servers={selectedTallySessionData?.servers ?? []} />
                </WizardStyles.AccordionDetails>
            </Accordion>
            <Accordion
                sx={{width: "100%"}}
                expanded={expandedExports["tally-download-package"]}
                onChange={() =>
                    setExpandedDataExports((prev: IExpanded) => ({
                        ...prev,
                        "tally-download-package": !prev["tally-download-package"],
                    }))
                }
            >
                <AccordionSummary>
                    <WizardStyles.AccordionTitle>
                        {t("tally.sendToTransmissionPackageServers")}
                    </WizardStyles.AccordionTitle>
                    <TallyStyles.StyledSpacing>
                        {transmissionLoading ? (
                            <CircularProgress />
                        ) : (
                            <ExportButton
                                aria-label="export election data"
                                aria-controls="export-menu"
                                aria-haspopup="true"
                                onClick={handleSendTransmissionPackage}
                            >
                                <span title={"Upload"}>{"Upload"}</span>
                            </ExportButton>
                        )}
                    </TallyStyles.StyledSpacing>
                </AccordionSummary>
            </Accordion>
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
                        {t("tally.transmissionPackageSignatures")}
                    </WizardStyles.AccordionTitle>
                </AccordionSummary>
                <WizardStyles.AccordionDetails style={{zIndex: 100}}>
                    <MiruSignatures
                        signatures={
                            selectedTallySessionData?.documents[
                                selectedTallySessionData?.documents.length - 1
                            ].signatures ?? []
                        }
                    />
                </WizardStyles.AccordionDetails>
            </Accordion>
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
                    <WizardStyles.AccordionTitle>
                        {t("tally.uploadTransmissionPackage")}
                    </WizardStyles.AccordionTitle>
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
            <Logs logs={selectedTallySessionData?.logs} />
        </>
    )
}
