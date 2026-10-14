// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {ServerCustodyError} from "@/services/serverCustody"
import {useServerCustodyProof} from "@/components/keys-ceremony/useServerCustodyProof"
import {IPermissions} from "@/types/keycloak"
import React, {useCallback, useContext, useEffect, useMemo, useState} from "react"
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
import {AUTHORIZE_TALLY_CUSTODY} from "@/queries/AuthorizeTallyCustody"
import {
    CUSTODY_PROOF_SIGNATURE_STATUS,
    type ICustodyProofSignatureStatusQuery,
    type ICustodyProofSignatureStatusVariables,
} from "@/queries/CustodyProofSignatureStatus"
import {useMutation, useQuery} from "@apollo/client"
import {
    ETallyKeyRestoreEligibility,
    ITallyExecutionStatus,
    ITallyTrusteeStatus,
} from "@/types/ceremonies"
import {Alert, Box, Typography} from "@mui/material"
import {
    AuthorizeTallyCustodyMutation,
    AuthorizeTallyCustodyMutationVariables,
    Sequent_Backend_Election,
    Sequent_Backend_Election_Event,
    Sequent_Backend_Tally_Session,
    Sequent_Backend_Tally_Session_Execution,
} from "@/gql/graphql"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {
    CustodyProofCheck,
    useCustodyProofSigning,
    type ICustodyProofAnswer,
    type ICustodyProofSubmission,
} from "@/components/keys-ceremony/useCustodyProofSigning"
import {getTallyTrusteeStatus} from "@/services/tallyCeremonyParticipation"
import {getTallyKeyRestoreEligibility} from "./utils"

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
    const trustee = authContext.trustee || authContext.username

    const [hasContinuedToStatus, setHasContinuedToStatus] = useState(false)
    const [selectedElections, setSelectedElections] = useState<string[]>([])
    const [selectedTrustees, setSelectedTrustees] = useState<boolean>(false)
    const [tally, setTally] = useState<Sequent_Backend_Tally_Session>()
    const serverCustody = useServerCustodyProof({
        election_event_id: record?.id || "",
        tally_session_id: tally?.id || tallyId || "",
    })
    const [verified, setVerified] = useState<boolean>(false)
    const [uploading, setUploading] = useState<boolean>(false)
    const [errors, setErrors] = useState<String | null>(null)
    const {globalSettings} = useContext(SettingsContext)
    const [isTallyCompleted, setIsTallyCompleted] = useState<boolean>(false)
    const {data} = useGetOne<Sequent_Backend_Tally_Session>(
        "sequent_backend_tally_session",
        {
            id: tallyId,
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

    const trusteeStatus = useMemo(
        () => getTallyTrusteeStatus(tallySessionExecutions?.[0], trustee),
        [trustee, tallySessionExecutions]
    )

    // A trustee who confirmed custody unsigned before the rule made trustees
    // sign contributes it again, signed.
    // The state holds the tally trustee's status.
    const custodyConfirmed =
        (trusteeStatus as string | null) === ITallyTrusteeStatus.CUSTODY_CONFIRMED
    const {data: signatureStatus, refetch: refetchSignatureStatus} = useQuery<
        ICustodyProofSignatureStatusQuery,
        ICustodyProofSignatureStatusVariables
    >(CUSTODY_PROOF_SIGNATURE_STATUS, {
        variables: {electionEventId: tally?.election_event_id ?? "", tallySessionId: tally?.id},
        skip:
            !custodyConfirmed ||
            !tally ||
            tally.execution_status === ITallyExecutionStatus.CANCELLED ||
            !!tally.is_execution_completed,
        fetchPolicy: "network-only",
    })
    const redoSigned =
        custodyConfirmed &&
        !!signatureStatus?.custody_proof_signature_status?.signature_needed &&
        !signatureStatus.custody_proof_signature_status.signed

    const eligibility = getTallyKeyRestoreEligibility(
        redoSigned ? ITallyTrusteeStatus.WAITING : trusteeStatus,
        tally?.execution_status
    )
    const page =
        eligibility === ETallyKeyRestoreEligibility.ALLOWED && !hasContinuedToStatus
            ? WizardSteps.Start
            : WizardSteps.Status

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

    const [authorizeTallyCustodyMutation] = useMutation<
        AuthorizeTallyCustodyMutation,
        AuthorizeTallyCustodyMutationVariables
    >(AUTHORIZE_TALLY_CUSTODY, {
        context: {headers: {"x-hasura-role": IPermissions.TRUSTEE_CEREMONY}},
    })
    const submit = useCallback(
        async (submission: ICustodyProofSubmission): Promise<ICustodyProofAnswer> => {
            const {data, errors} = await authorizeTallyCustodyMutation({
                variables: {
                    electionEventId: tally?.election_event_id ?? "",
                    tallySessionId: tally?.id ?? "",
                    ...submission,
                },
            })
            if (errors) throw new Error(errors.toString())
            return data?.authorize_tally_custody ?? {is_valid: false}
        },
        [authorizeTallyCustodyMutation, tally?.election_event_id, tally?.id]
    )
    const custodyProofSigning = useCustodyProofSigning({
        submit,
        onRecorded: useCallback(() => {
            setVerified(true)
            // A redone contribution is now signed.
            if (redoSigned) refetchSignatureStatus().catch(() => undefined)
        }, [redoSigned, refetchSignatureStatus]),
        onFailed: useCallback(
            (error: string) => setErrors(t("signing.custodyProof.failed", {error})),
            [t]
        ),
    })

    const uploadCustodyProof = async (files: FileList | null) => {
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
                fileReader.readAsText(file)
            })
        }
        try {
            const fileContent = await readFileContent(firstFile)
            if (fileContent == null) {
                setErrors(t("keysGeneration.checkStep.noFileSelected"))
                return
            }
            setUploading(true)
            const checked = await custodyProofSigning.check(
                await serverCustody.resolve(fileContent)
            )
            setUploading(false)
            if (checked === CustodyProofCheck.Invalid) {
                setErrors(t("keysGeneration.checkStep.errorUploading", {error: "empty"}))
            } else if (checked === CustodyProofCheck.Verified) {
                setVerified(true)
            }
        } catch (exception) {
            if (exception instanceof ServerCustodyError && exception.reason === "cancelled") return
            setUploading(false)
            setErrors(
                exception instanceof ServerCustodyError
                    ? exception.message
                    : t("keysGeneration.checkStep.errorUploading", {error: String(exception)})
            )
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

                    {tally?.id && (
                        <Box sx={{my: 2}}>
                            <Typography variant="body2" sx={{m: 0}}>
                                {t("tally.sessionId")}
                            </Typography>
                            <Typography
                                component="code"
                                sx={{
                                    display: "block",
                                    overflowWrap: "anywhere",
                                    userSelect: "all",
                                    m: 0,
                                }}
                            >
                                {tally.id}
                            </Typography>
                        </Box>
                    )}

                    {page === WizardSteps.Start && (
                        <>
                            <ElectionHeader
                                title={"tally.ceremonyTitle"}
                                subtitle={"tally.ceremonySubTitle"}
                            />

                            <TallyElectionsList
                                elections={elections}
                                electionEventPresentation={record?.presentation}
                                electionEventId={record?.id}
                                disabled={true}
                                update={(elections) => setSelectedElections(elections)}
                                keysCeremonyId={data?.keys_ceremony_id ?? null}
                            />

                            <Box>
                                <ElectionHeader
                                    title={"tally.trusteeTitle"}
                                    subtitle={"keysGeneration.checkStep.subtitle"}
                                />
                                {redoSigned && !verified ? (
                                    <Typography variant="body1">
                                        {t("signing.custodyProof.redo")}
                                    </Typography>
                                ) : null}

                                <Alert severity="info" sx={{mb: 2}}>
                                    {t("keysGeneration.serverCustody.tallyInstructions")}
                                </Alert>
                                <DropFile
                                    handleFiles={uploadCustodyProof}
                                    formatLabel={t("keysGeneration.serverCustody.fileFormat")}
                                />

                                <WizardStyles.StatusBox>
                                    {uploading ? <WizardStyles.DownloadProgress /> : null}
                                    {errors ? (
                                        <WizardStyles.ErrorMessage variant="body2">
                                            {errors}
                                        </WizardStyles.ErrorMessage>
                                    ) : null}
                                    {serverCustody.waiting && !errors ? (
                                        <Typography role="status">
                                            {t("keysGeneration.serverCustody.waiting")}
                                        </Typography>
                                    ) : null}
                                    {custodyProofSigning.waiting && !errors ? (
                                        <Typography variant="body1">
                                            {t("signing.custodyProof.signing")}
                                        </Typography>
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
                                electionEventPresentation={record?.presentation}
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
                            onClick={() => setHasContinuedToStatus(true)}
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
