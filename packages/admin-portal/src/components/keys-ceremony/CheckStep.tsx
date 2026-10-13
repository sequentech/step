// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {IPermissions} from "@/types/keycloak"
import {AuthContext} from "@/providers/AuthContextProvider"
import {ServerCustodyError} from "@/services/serverCustody"
import {useServerCustodyProof} from "./useServerCustodyProof"
import {useMutation} from "@apollo/client"
import React, {useCallback, useContext, useState} from "react"
import {Alert, Typography} from "@mui/material"
import ArrowForwardIosIcon from "@mui/icons-material/ArrowForwardIos"
import ArrowBackIosIcon from "@mui/icons-material/ArrowBackIos"
import {Trans, useTranslation} from "react-i18next"

import {
    ConfirmKeyCustodyMutation,
    ConfirmKeyCustodyMutationVariables,
    Sequent_Backend_Election_Event,
    Sequent_Backend_Keys_Ceremony,
} from "@/gql/graphql"
import {WizardStyles} from "@/components/styles/WizardStyles"
import {CONFIRM_KEY_CUSTODY} from "@/queries/ConfirmKeyCustody"
import {DropFile} from "@sequentech/ui-essentials"
import {
    CustodyProofCheck,
    useCustodyProofSigning,
    type ICustodyProofAnswer,
    type ICustodyProofSubmission,
} from "./useCustodyProofSigning"

export interface DownloadStepProps {
    electionEvent: Sequent_Backend_Election_Event
    currentCeremony: Sequent_Backend_Keys_Ceremony
    goNext: () => void
    goBack: () => void
}

export const CheckStep: React.FC<DownloadStepProps> = ({
    electionEvent,
    currentCeremony,
    goNext,
    goBack,
}) => {
    const {t} = useTranslation()
    const authContext = useContext(AuthContext)
    const serverCustody = useServerCustodyProof({
        election_event_id: electionEvent.id,
        keys_ceremony_id: currentCeremony.id,
    })
    const [verified, setVerified] = useState<boolean>(false)
    const [uploading, setUploading] = useState<boolean>(false)
    const [errors, setErrors] = useState<String | null>(null)

    const [confirmKeyCustodyMutation] = useMutation<
        ConfirmKeyCustodyMutation,
        ConfirmKeyCustodyMutationVariables
    >(CONFIRM_KEY_CUSTODY, {context: {headers: {"x-hasura-role": IPermissions.TRUSTEE_CEREMONY}}})
    const submit = useCallback(
        async (submission: ICustodyProofSubmission): Promise<ICustodyProofAnswer> => {
            const {data, errors} = await confirmKeyCustodyMutation({
                variables: {
                    electionEventId: electionEvent.id,
                    keysCeremonyId: currentCeremony.id,
                    ...submission,
                },
            })
            if (errors) throw new Error(errors.toString())
            return data?.confirm_key_custody ?? {is_valid: false}
        },
        [confirmKeyCustodyMutation, electionEvent.id, currentCeremony.id]
    )
    const custodyProofSigning = useCustodyProofSigning({
        submit,
        onRecorded: useCallback(() => setVerified(true), []),
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
        <>
            <WizardStyles.ContentBox>
                <WizardStyles.StepHeader variant="h4">
                    {t("keysGeneration.checkStep.title")}
                </WizardStyles.StepHeader>
                <WizardStyles.MainContent>
                    <Alert severity="info">
                        {t("keysGeneration.serverCustody.uploadInstructions")}
                    </Alert>
                    <Typography variant="body1">
                        <Trans
                            i18nKey="keysGeneration.checkStep.subtitle"
                            values={{name: authContext.username}}
                        ></Trans>
                    </Typography>

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
                </WizardStyles.MainContent>
            </WizardStyles.ContentBox>

            <WizardStyles.Toolbar>
                <WizardStyles.BackButton color="info" onClick={goBack}>
                    <ArrowBackIosIcon />
                    {t("common.label.back")}
                </WizardStyles.BackButton>
                <WizardStyles.NextButton disabled={!verified} color="info" onClick={goNext}>
                    <ArrowForwardIosIcon />
                    {t("common.label.next")}
                </WizardStyles.NextButton>
            </WizardStyles.Toolbar>
        </>
    )
}
