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

// interface IMiruExportWizard {
// 	expandedResults: () => void
// 	resultsEvent:
// 	documents:
// 	setExpandedResults:
// 	transmissionLoading:
// 	handleSendTransmissionPackage:
// 	selectedTallySessionData: IMiru
// 	uploading: boolean
// 	errors: string
// 	handleUploadSignature: ()=>void
// }

export const MiruExportWizard = ({
    expandedResults,
    resultsEvent,
    documents,
    setExpandedResults,
    transmissionLoading,
    handleSendTransmissionPackage,
    selectedTallySessionData,
    uploading,
    errors,
    handleUploadSignature,
}: any) => {
    return (
        <>
            <Accordion
                sx={{width: "100%"}}
                expanded={expandedResults["tally-download-package"]}
                onChange={() =>
                    setExpandedResults((prev: IExpanded) => ({
                        ...prev,
                        "tally-download-package": !prev["tally-download-package"],
                    }))
                }
            >
                <AccordionSummary>
                    <WizardStyles.AccordionTitle>Download Package</WizardStyles.AccordionTitle>
                    <TallyStyles.StyledSpacing>
                        {resultsEvent?.[0] && documents ? (
                            <MiruPackageDownload
                                documents={selectedTallySessionData.documents}
                                electionEventId={resultsEvent?.[0].election_event_id}
                            />
                        ) : null}
                    </TallyStyles.StyledSpacing>
                </AccordionSummary>
            </Accordion>
            <Accordion
                sx={{width: "100%"}}
                expanded={expandedResults["tally-miru-servers"]}
                onChange={() =>
                    setExpandedResults((prev: IExpanded) => ({
                        ...prev,
                        "tally-miru-servers": !prev["tally-miru-servers"],
                    }))
                }
            >
                <AccordionSummary expandIcon={<ExpandMoreIcon id="tally-miru-servers" />}>
                    <WizardStyles.AccordionTitle>Servers</WizardStyles.AccordionTitle>
                </AccordionSummary>
                <WizardStyles.AccordionDetails style={{zIndex: 100}}>
                    <MiruServers servers={selectedTallySessionData.servers} />
                </WizardStyles.AccordionDetails>
            </Accordion>
            <Accordion
                sx={{width: "100%"}}
                expanded={expandedResults["tally-download-package"]}
                onChange={() =>
                    setExpandedResults((prev: IExpanded) => ({
                        ...prev,
                        "tally-download-package": !prev["tally-download-package"],
                    }))
                }
            >
                <AccordionSummary>
                    <WizardStyles.AccordionTitle>Send to Servers</WizardStyles.AccordionTitle>
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
                expanded={expandedResults["tally-miru-signatures"]}
                onChange={() =>
                    setExpandedResults((prev: IExpanded) => ({
                        ...prev,
                        "tally-miru-signatures": !prev["tally-miru-signatures"],
                    }))
                }
            >
                <AccordionSummary expandIcon={<ExpandMoreIcon id="tally-miru-signatures" />}>
                    <WizardStyles.AccordionTitle>Signatures</WizardStyles.AccordionTitle>
                </AccordionSummary>
                <WizardStyles.AccordionDetails style={{zIndex: 100}}>
                    <MiruSignatures
                        signatures={
                            selectedTallySessionData.documents[
                                selectedTallySessionData.documents.length - 1
                            ].signatures
                        }
                    />
                </WizardStyles.AccordionDetails>
            </Accordion>
            <Accordion
                sx={{width: "100%"}}
                expanded={expandedResults["tally-upload"]}
                onChange={() =>
                    setExpandedResults((prev: IExpanded) => ({
                        ...prev,
                        "tally-upload": !prev["tally-upload"],
                    }))
                }
            >
                <AccordionSummary>
                    <WizardStyles.AccordionTitle>Upload</WizardStyles.AccordionTitle>
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
            <Logs logs={selectedTallySessionData.logs} />
        </>
    )
}
