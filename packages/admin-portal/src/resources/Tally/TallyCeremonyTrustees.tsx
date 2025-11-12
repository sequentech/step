// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useState} from "react"
import Button from "@mui/material/Button"
import {BreadCrumbSteps, BreadCrumbStepsVariant, DropFile} from "@sequentech/ui-essentials"
import ChevronRightIcon from "@mui/icons-material/ChevronRight"
import ArrowBackIosIcon from "@mui/icons-material/ArrowBackIos"
import {useTranslation} from "react-i18next"
import ElectionHeader from "@/components/ElectionHeader"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {styled} from "@mui/material/styles"
import {TallyElectionsList} from "./TallyElectionsList"
import {TallyTrusteesList} from "./TallyTrusteesList"
import {TallyStyles} from "@/components/styles/TallyStyles"
import {useGetList, useGetOne, useRecordContext} from "react-admin"
import {WizardStyles} from "@/components/styles/WizardStyles"
import {RESTORE_PRIVATE_KEY} from "@/queries/RestorePrivateKey"
import {useMutation} from "@apollo/client"
import {
    ICeremonyStatus,
    ITallyExecutionStatus,
    ITallyTrusteeStatus,
    ITrusteeStatus,
} from "@/types/ceremonies"
import {Box} from "@mui/material"
import {
    RestorePrivateKeyMutation,
    Sequent_Backend_Election,
    Sequent_Backend_Election_Event,
    Sequent_Backend_Tally_Session,
    Sequent_Backend_Tally_Session_Execution,
} from "@/gql/graphql"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {SettingsContext} from "@/providers/SettingsContextProvider"

const WizardSteps = {
    Start: 0,
    Status: 1,
}

export const TallyCeremonyTrustees: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()

    const {t} = useTranslation()
    const {tallyId, setTallyId} = useElectionEventTallyStore()
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)

    const [page, setPage] = useState<number>(WizardSteps.Start)
    const [selectedElections, setSelectedElections] = useState<string[]>([])
    const [selectedTrustees, setSelectedTrustees] = useState<boolean>(false)
    const [tally, setTally] = useState<Sequent_Backend_Tally_Session>()
    const [verified, setVerified] = useState<boolean>(false)
    const [uploading, setUploading] = useState<boolean>(false)
    const [errors, setErrors] = useState<String | null>(null)
    const [trusteeStatus, setTrusteeStatus] = useState<ITrusteeStatus | null>(null)
    const {globalSettings} = useContext(SettingsContext)
    const [isTallyCompleted, setIsTallyCompleted] = useState<boolean>(false)

    const {data} = useGetOne<Sequent_Backend_Tally_Session>(
        "sequent_backend_tally_session",
        {
            id: tallyId,
        },
        {
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
        }
    )

    // TODO: fix the "perPage 9999"
    const {data: elections} = useGetList<Sequent_Backend_Election>("sequent_backend_election", {
        pagination: {page: 1, perPage: 9999},
        filter: {
            election_event_id: record?.id,
            tenant_id: tenantId,
            id: tallyId
                ? {
                      format: "hasura-raw-query",
                      value: {
                          _in: tally?.election_ids ?? [],
                      },
                  }
                : undefined,
        },
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
            refetchInterval: isTallyCompleted
                ? undefined
                : globalSettings.QUERY_FAST_POLL_INTERVAL_MS,
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
        }
    )

    useEffect(() => {
        if (data?.is_execution_completed && !isTallyCompleted) {
            setIsTallyCompleted(true)
        }
    }, [data?.is_execution_completed, isTallyCompleted])

    useEffect(() => {
        if (data) {
            setTally(data)
        }
    }, [data])

    useEffect(() => {
        if (tallySessionExecutions) {
            const username = authContext?.username
            const ceremonyStatus: ICeremonyStatus | undefined = tallySessionExecutions?.[0]?.status
            const trusteeStatus = ceremonyStatus?.trustees.find(
                (item) => item.name === username
            )?.status
            setTrusteeStatus(trusteeStatus ?? null)
        }
    }, [tallySessionExecutions])

    useEffect(() => {
        setPage(
            !trusteeStatus && tally?.execution_status !== ITallyExecutionStatus.CANCELLED
                ? WizardSteps.Start
                : trusteeStatus === ITrusteeStatus.WAITING &&
                    tally?.execution_status !== ITallyExecutionStatus.CANCELLED
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
        <TallyStyles.WizardContainer>
            <TallyStyles.ContentWrapper>
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
                                elections={elections}
                                electionEventId={record?.id}
                                disabled={true}
                                update={(elections) => setSelectedElections(elections)}
                                keysCeremonyId={data?.keys_ceremony_id ?? null}
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
                                elections={elections}
                                electionEventId={record?.id}
                                disabled={true}
                                update={(elections) => setSelectedElections(elections)}
                                keysCeremonyId={data?.keys_ceremony_id ?? null}
                            />

                            <TallyTrusteesList
                                tally={tally}
                                update={(trustees) => setSelectedTrustees(trustees)}
                                tallySessionExecutions={tallySessionExecutions}
                            />
                        </>
                    )}
                </WizardStyles.WizardWrapper>
            </TallyStyles.ContentWrapper>

            <TallyStyles.FooterContainer>
                <TallyStyles.StyledFooter>
                    <CancelButton className="list-actions" onClick={() => setTallyId(null)}>
                        <ArrowBackIosIcon />
                        {t("tally.common.cancel")}
                    </CancelButton>
                    {page < WizardSteps.Status && (
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
            </TallyStyles.FooterContainer>
        </TallyStyles.WizardContainer>
    )
}
