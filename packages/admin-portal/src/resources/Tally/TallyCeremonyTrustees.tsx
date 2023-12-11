// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import Button from "@mui/material/Button"
import {
    BreadCrumbSteps,
    BreadCrumbStepsVariant,
    Dialog,
    DropFile,
    IconButton,
} from "@sequentech/ui-essentials"
import ChevronRightIcon from "@mui/icons-material/ChevronRight"
import {useTranslation} from "react-i18next"
import ElectionHeader from "@/components/ElectionHeader"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import styled from "@emotion/styled"
import {Accordion, AccordionDetails, AccordionSummary} from "@mui/material"
import {ElectionStyles} from "@/components/styles/ElectionStyles"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"
import {ListActions} from "@/components/ListActions"
import {TallyElectionsList} from "./TallyElectionsList"
import {TallyTrusteesList} from "./TallyTrusteesList"
import {TallyStyles} from "@/components/styles/TallyStyles"
import {TallyStartDate} from "./TallyStartDate"
import {TallyElectionsProgress} from "./TallyElectionsProgress"
import {TallyElectionsResults} from "./TallyElectionsResults"
import {TallyResults} from "./TallyResults"
import {TallyLogs} from "./TallyLogs"
import {useGetOne, useNotify} from "react-admin"
import {WizardStyles} from "@/components/styles/WizardStyles"
import {UPDATE_TALLY_CEREMONY} from "@/queries/UpdateTallyCeremony"
import {useMutation} from "@apollo/client"
import {ITallyExecutionStatus} from "@/types/ceremonies"
import {faKey} from "@fortawesome/free-solid-svg-icons"
import {Box} from "@mui/material"
import {Sequent_Backend_Area, Sequent_Backend_Tally_Session} from "@/gql/graphql"
import {AuthContext, AuthContextValues} from "@/providers/AuthContextProvider"

// interface TallyCeremonyTrusteesProps {
//     completed: boolean
// }

const WizardSteps = {
    Start: 0,
    Status: 1,
}

export const TallyCeremonyTrustees: React.FC = () => {
    const {t} = useTranslation()
    const [tallyId, setTallyId] = useElectionEventTallyStore()
    const notify = useNotify()

    const [page, setPage] = useState<number>(WizardSteps.Start)
    const [selectedElections, setSelectedElections] = useState<string[]>([])
    const [selectedTrustees, setSelectedTrustees] = useState<string[]>([])
    const [tally, setTally] = useState<Sequent_Backend_Tally_Session>()
    const [verified, setVerified] = useState<boolean>(false)
    const [uploading, setUploading] = useState<boolean>(false)
    const [errors, setErrors] = useState<String | null>(null)

    const {data} = useGetOne<Sequent_Backend_Tally_Session>("sequent_backend_tally_session", {
        id: tallyId,
    })

    useEffect(() => {
        if (data) {
            // TODO: uncomment to control the screen state depending on tally_execution_status
            // setPage(
            //     data?.execution_status === ITallyExecutionStatus.NOT_STARTED ||
            //         data?.execution_status === ITallyExecutionStatus.STARTED
            //         ? WizardSteps.Start
            //         : WizardSteps.Status
            // )
            if (tally?.last_updated_at !== data.last_updated_at) {
                setTally(data)
            }
        }
    }, [data])

    const CancelButton = styled(Button)`
        background-color: ${({theme}) => theme.palette.white};
        color: ${({theme}) => theme.palette.brandColor};
        border-color: ${({theme}) => theme.palette.brandColor};
        padding: 0 4rem;

        &:hover {
            background-color: ${({theme}) => theme.palette.brandColor};
        }
    `

    const NextButton = styled(Button)`
        background-color: ${({theme}) => theme.palette.brandColor};
        color: ${({theme}) => theme.palette.white};
        border-color: ${({theme}) => theme.palette.brandColor};
        padding: 0 4rem;

        &:hover {
            background-color: ${({theme}) => theme.palette.white};
            color: ${({theme}) => theme.palette.brandColor};
        }
    `

    // const [checkPrivateKeysMutation] = useMutation<CheckPrivateKeyMutation>(CHECK_PRIVATE_KEY)
    const uploadPrivateKey = async (files: FileList | null) => {
        //TODO:i todo upload key
        console.log("TallyCeremonyTrustees :: uploadPrivateKey :: files", files)
        setVerified(true)

        // setErrors(null)
        // setVerified(false)
        // setUploading(false)
        // if (!files || files.length === 0) {
        //     setErrors(t("keysGeneration.checkStep.noFileSelected"))
        //     return
        // }
        // const firstFile = files[0]
        // const readFileContent = (file: File) => {
        //     return new Promise<string>((resolve, reject) => {
        //         const fileReader = new FileReader()
        //         fileReader.onload = () => resolve(fileReader.result as string)
        //         fileReader.onerror = (error) => reject(error)
        //         // Read the file as a data URL (base64 encoded string)
        //         fileReader.readAsText(file)
        //     })
        // }
        // try {
        //     const fileContent = await readFileContent(firstFile)
        //     console.log(`uploadPrivateKey(): fileContent: ${fileContent}`)
        //     if (fileContent == null) {
        //         setErrors(t("keysGeneration.checkStep.noFileSelected"))
        //         return
        //     }
        //     setUploading(true)
        //     const {data, errors} = await checkPrivateKeysMutation({
        //         variables: {
        //             electionEventId: electionEvent.id,
        //             keysCeremonyId: currentCeremony.id,
        //             privateKeyBase64: fileContent,
        //         },
        //     })
        //     setUploading(false)
        //     if (errors) {
        //         setErrors(t("keysGeneration.checkStep.errorUploading", {error: errors.toString()}))
        //         return
        //     } else {
        //         const isValid = data?.check_private_key?.is_valid
        //         if (!isValid) {
        //             setErrors(t("keysGeneration.checkStep.errorUploading", {error: "empty"}))
        //             return
        //         }
        //         setVerified(true)
        //     }
        // } catch (exception: any) {
        //     setUploading(false)
        //     setErrors(t("keysGeneration.checkStep.errorUploading", {error: exception.toString()}))
        // }
    }

    return (
        <>
            <WizardStyles.WizardWrapper>
                <TallyStyles.StyledHeader>
                    <BreadCrumbSteps
                        labels={["tally.breadcrumbSteps.start", "tally.breadcrumbSteps.finish"]}
                        selected={page}
                        variant={BreadCrumbStepsVariant.Circle}
                        colorPreviousSteps={true}
                    />
                </TallyStyles.StyledHeader>

                {page === WizardSteps.Start && (
                    <>
                        <ElectionHeader
                            title={t("tally.ceremonyTitle")}
                            subtitle={t("tally.ceremonySubTitle")}
                        />

                        <TallyElectionsList
                            update={(elections) => setSelectedElections(elections)}
                        />

                        <Box>
                            <ElectionHeader
                                title={t("tally.trusteeTitle")}
                                subtitle={t("tally.trusteeSubTitle")}
                            />

                            <DropFile handleFiles={uploadPrivateKey} />

                            <WizardStyles.StatusBox>
                                {uploading ? <WizardStyles.DownloadProgress /> : null}
                                {errors ? (
                                    <WizardStyles.ErrorMessage variant="body2">
                                        {errors}
                                    </WizardStyles.ErrorMessage>
                                ) : null}
                                {verified && (
                                    <WizardStyles.SucessMessage variant="body1">
                                        {t("keysGeneration.checkStep.verified")}
                                    </WizardStyles.SucessMessage>
                                )}
                            </WizardStyles.StatusBox>
                        </Box>
                    </>
                )}

                {page === WizardSteps.Status && (
                    <>
                        <ElectionHeader
                            title={t("tally.ceremonyTitle")}
                            subtitle={t("tally.ceremonySubTitle")}
                        />

                        <TallyElectionsList
                            update={(elections) => setSelectedElections(elections)}
                        />

                        <TallyStyles.StyledFooter>
                            <ElectionHeader
                                title={t("tally.trusteeTallyTitle")}
                                subtitle={t("tally.trusteeTallySubTitle")}
                            />
                        </TallyStyles.StyledFooter>

                        <Box
                            sx={{
                                width: "100%",
                                display: "flex",
                                justifyContent: "flex-end",
                            }}
                        >
                            <IconButton
                                icon={faKey}
                                sx={{
                                    color:
                                        tally?.execution_status === ITallyExecutionStatus.CONNECTED
                                            ? "#43E3A1"
                                            : "#d32f2f",
                                }}
                            />
                        </Box>

                        <TallyTrusteesList update={(trustees) => setSelectedTrustees(trustees)} />
                    </>
                )}

                <TallyStyles.StyledFooter>
                    <CancelButton className="list-actions" onClick={() => setTallyId(null)}>
                        {t("tally.common.cancel")}
                    </CancelButton>
                    {page === WizardSteps.Start && (
                        <NextButton
                            color="primary"
                            onClick={() => setPage(WizardSteps.Status)}
                            disabled={!verified}
                        >
                            <>
                                {t("tally.common.next")}
                                <ChevronRightIcon />
                            </>
                        </NextButton>
                    )}
                </TallyStyles.StyledFooter>
            </WizardStyles.WizardWrapper>
        </>
    )
}
