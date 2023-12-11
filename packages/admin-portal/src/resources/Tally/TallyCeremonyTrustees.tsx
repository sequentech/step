// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useState} from "react"
import Button from "@mui/material/Button"
import {
    BreadCrumbSteps,
    BreadCrumbStepsVariant,
    DropFile,
    IconButton,
} from "@sequentech/ui-essentials"
import ChevronRightIcon from "@mui/icons-material/ChevronRight"
import {useTranslation} from "react-i18next"
import ElectionHeader from "@/components/ElectionHeader"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import styled from "@emotion/styled"
import {TallyElectionsList} from "./TallyElectionsList"
import {TallyTrusteesList} from "./TallyTrusteesList"
import {TallyStyles} from "@/components/styles/TallyStyles"
import {useGetList, useGetOne} from "react-admin"
import {WizardStyles} from "@/components/styles/WizardStyles"
import {RESTORE_PRIVATE_KEY} from "@/queries/RestorePrivateKey"
import {useMutation} from "@apollo/client"
import {ITallyExecutionStatus, ITallyTrusteeStatus} from "@/types/ceremonies"
import {faKey} from "@fortawesome/free-solid-svg-icons"
import {Box} from "@mui/material"
import {RestorePrivateKeyMutation, Sequent_Backend_Tally_Session, Sequent_Backend_Tally_Session_Execution} from "@/gql/graphql"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"

const WizardSteps = {
    Start: 0,
    Status: 1,
}

export const TallyCeremonyTrustees: React.FC = () => {
    const {t} = useTranslation()
    const [tallyId, setTallyId] = useElectionEventTallyStore()
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)

    const [page, setPage] = useState<number>(WizardSteps.Start)
    const [selectedElections, setSelectedElections] = useState<string[]>([])
    const [selectedTrustees, setSelectedTrustees] = useState<string[]>([])
    const [tally, setTally] = useState<Sequent_Backend_Tally_Session>()
    const [verified, setVerified] = useState<boolean>(false)
    const [uploading, setUploading] = useState<boolean>(false)
    const [errors, setErrors] = useState<String | null>(null)
    const [trusteeStatus, setTrusteeStatus] = useState<String | null>(null)

    const {data} = useGetOne<Sequent_Backend_Tally_Session>("sequent_backend_tally_session", {
        id: tallyId,
    })

    const {data: tallySessionExecutions} = useGetList<Sequent_Backend_Tally_Session_Execution>(
        "sequent_backend_tally_session_execution",
        {
            pagination: {page: 1, perPage: 1},
            sort: {field: "created_at", order: "DESC"},
            filter: {
                tally_session_id: tallyId,
                tenant_id: tenantId,
            },
        },
        {
            refetchInterval: 5000,
        }
    )

    useEffect(() => {
        if (data) {
            if (tally?.last_updated_at !== data.last_updated_at) {
                setTally(data)
            }
        }
    }, [data])

    useEffect(() => {
        if (tallySessionExecutions) {
            const username = authContext?.username
            const trusteeStatus = tallySessionExecutions?.[0]?.status?.trustees.find(
                (item: any) => item.name === username
            )?.status
            setTrusteeStatus(trusteeStatus)
        }
    }, [tallySessionExecutions])

    useEffect(() => {
        setPage(
            !trusteeStatus
                ? WizardSteps.Start
                : trusteeStatus === ITallyTrusteeStatus.WAITING ||
                  trusteeStatus === ITallyTrusteeStatus.KEY_RESTORED
                ? WizardSteps.Start
                : WizardSteps.Status
        )
    }, [trusteeStatus])

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

    const [restorePrivateKeyMutation] = useMutation<RestorePrivateKeyMutation>(RESTORE_PRIVATE_KEY)
    const uploadPrivateKey = async (files: FileList | null) => {
        //TODO:i todo upload key
        console.log("TallyCeremonyTrustees :: uploadPrivateKey :: files", files)
        setVerified(true)

        setErrors(null)
        setVerified(false)
        setUploading(false)
        if (!files || files.length === 0) {
            setErrors(t("keysGeneration.checkStep.noFileSelected"))
            return
        }
        const firstFile = files[0]
        const readFileContent = (file: File) => {
            return new Promise<string>((resolve, reject) => {
                const fileReader = new FileReader()
                fileReader.onload = () => resolve(fileReader.result as string)
                fileReader.onerror = (error) => reject(error)
                // Read the file as a data URL (base64 encoded string)
                fileReader.readAsText(file)
            })
        }
        try {
            const fileContent = await readFileContent(firstFile)
            console.log(`uploadPrivateKey(): fileContent: ${fileContent}`)
            if (fileContent == null) {
                setErrors(t("keysGeneration.checkStep.noFileSelected"))
                return
            }
            setUploading(true)
            const {data, errors} = await restorePrivateKeyMutation({
                variables: {
                    electionEventId: tally?.election_event_id,
                    tallySessionId: tally?.id,
                    privateKeyBase64: fileContent,
                },
            })
            setUploading(false)
            if (errors) {
                setErrors(t("keysGeneration.checkStep.errorUploading", {error: errors.toString()}))
                return
            } else {
                const isValid = data?.restore_private_key?.is_valid
                if (!isValid) {
                    setErrors(t("keysGeneration.checkStep.errorUploading", {error: "empty"}))
                    return
                }
                setVerified(true)
            }
        } catch (exception: any) {
            setUploading(false)
            setErrors(t("keysGeneration.checkStep.errorUploading", {error: exception.toString()}))
        }
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
                            title={"tally.ceremonyTitle"}
                            subtitle={"tally.ceremonySubTitle"}
                        />

                        <TallyElectionsList
                            update={(elections) => setSelectedElections(elections)}
                        />

                        <Box>
                            <ElectionHeader
                                title={"tally.trusteeTitle"}
                                subtitle={"tally.trusteeSubTitle"}
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
                            title={"tally.ceremonyTitle"}
                            subtitle={"tally.ceremonySubTitle"}
                        />

                        <TallyElectionsList
                            update={(elections) => setSelectedElections(elections)}
                        />

                        <TallyStyles.StyledFooter>
                            <ElectionHeader
                                title={"tally.trusteeTallyTitle"}
                                subtitle={"tally.trusteeTallySubTitle"}
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
