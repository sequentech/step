// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useMutation} from "@apollo/client"
import React, {useCallback, useContext, useState} from "react"
import {Typography} from "@mui/material"
import ArrowForwardIosIcon from "@mui/icons-material/ArrowForwardIos"
import ArrowBackIosIcon from "@mui/icons-material/ArrowBackIos"
import {Trans, useTranslation} from "react-i18next"

import {
    CheckPrivateKeyMutation,
    CheckPrivateKeyMutationVariables,
    Sequent_Backend_Election_Event,
    Sequent_Backend_Keys_Ceremony,
} from "@/gql/graphql"
import {AuthContext} from "@/providers/AuthContextProvider"
import {WizardStyles} from "@/components/styles/WizardStyles"
import {CHECK_PRIVATE_KEY} from "@/queries/CheckPrivateKey"
import {DropFile} from "@sequentech/ui-essentials"
import {
    KeyShareCheck,
    useKeyShareSigning,
    type IKeyShareAnswer,
    type IKeyShareSubmission,
} from "./useKeyShareSigning"

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
    const [verified, setVerified] = useState<boolean>(false)
    const [uploading, setUploading] = useState<boolean>(false)
    const [errors, setErrors] = useState<String | null>(null)

    const [checkPrivateKeysMutation] = useMutation<
        CheckPrivateKeyMutation,
        CheckPrivateKeyMutationVariables
    >(CHECK_PRIVATE_KEY)
    const submit = useCallback(
        async (submission: IKeyShareSubmission): Promise<IKeyShareAnswer> => {
            const {data, errors} = await checkPrivateKeysMutation({
                variables: {
                    electionEventId: electionEvent.id,
                    keysCeremonyId: currentCeremony.id,
                    ...submission,
                },
            })
            if (errors) throw new Error(errors.toString())
            return data?.check_private_key ?? {is_valid: false}
        },
        [checkPrivateKeysMutation, electionEvent.id, currentCeremony.id]
    )
    const keyShareSigning = useKeyShareSigning({
        submit,
        onRecorded: useCallback(() => setVerified(true), []),
        onFailed: useCallback(
            (error: string) => setErrors(t("signing.keyShare.failed", {error})),
            [t]
        ),
    })

    const uploadPrivateKey = async (files: FileList | null) => {
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
            if (fileContent == null) {
                setErrors(t("keysGeneration.checkStep.noFileSelected"))
                return
            }
            setUploading(true)
            const checked = await keyShareSigning.check(fileContent)
            setUploading(false)
            if (checked === KeyShareCheck.Invalid) {
                setErrors(t("keysGeneration.checkStep.errorUploading", {error: "empty"}))
            } else if (checked === KeyShareCheck.Verified) {
                setVerified(true)
            }
        } catch (exception) {
            setUploading(false)
            setErrors(t("keysGeneration.checkStep.errorUploading", {error: String(exception)}))
        }
    }
    return (
        <>
            <WizardStyles.ContentBox>
                <WizardStyles.StepHeader variant="h4">
                    {t("keysGeneration.checkStep.title")}
                </WizardStyles.StepHeader>
                <WizardStyles.MainContent>
                    <Typography variant="body1">
                        <Trans
                            i18nKey="keysGeneration.checkStep.subtitle"
                            values={{name: authContext.username}}
                        ></Trans>
                    </Typography>

                    <DropFile handleFiles={uploadPrivateKey} />
                    <WizardStyles.StatusBox>
                        {uploading ? <WizardStyles.DownloadProgress /> : null}
                        {errors ? (
                            <WizardStyles.ErrorMessage variant="body2">
                                {errors}
                            </WizardStyles.ErrorMessage>
                        ) : null}
                        {keyShareSigning.waiting && !errors ? (
                            <Typography variant="body1">{t("signing.keyShare.signing")}</Typography>
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
