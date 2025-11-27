// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useApolloClient} from "@apollo/client"
import React, {useContext, useState} from "react"
import {Typography, TextField} from "@mui/material"
import ArrowForwardIosIcon from "@mui/icons-material/ArrowForwardIos"
import ArrowBackIosIcon from "@mui/icons-material/ArrowBackIos"
import {Trans, useTranslation} from "react-i18next"

import {
    Sequent_Backend_Election_Event,
    Sequent_Backend_Keys_Ceremony,
} from "@/gql/graphql"
import {AuthContext} from "@/providers/AuthContextProvider"
import {WizardStyles} from "@/components/styles/WizardStyles"
import {TrusteeWasmService, IBraidWasmModule} from "@/services/TrusteeWasmService"
import * as BraidWasm from "braid-wasm"

import {DropFile} from "@sequentech/ui-essentials"

export interface WasmCheckStepProps {
    electionEvent: Sequent_Backend_Election_Event
    currentCeremony: Sequent_Backend_Keys_Ceremony
    goNext: () => void
    goBack: () => void
}

export const WasmCheckStep: React.FC<WasmCheckStepProps> = ({
    electionEvent,
    currentCeremony: _currentCeremony,
    goNext,
    goBack,
}) => {
    const {t} = useTranslation()
    const authContext = useContext(AuthContext)
    const apolloClient = useApolloClient()
    const wasmService = React.useMemo(
        () => new TrusteeWasmService(BraidWasm as unknown as IBraidWasmModule, apolloClient),
        [apolloClient]
    )

    const [verified, setVerified] = useState<boolean>(false)
    const [uploading, setUploading] = useState<boolean>(false)
    const [errors, setErrors] = useState<string | null>(null)
    const [passphrase, setPassphrase] = useState<string>("")

    const uploadPrivateKey = async (files: FileList | null) => {
        setErrors(null)
        setVerified(false)
        setUploading(false)

        if (!files || files.length === 0) {
            setErrors(t("keysGeneration.checkStep.noFileSelected"))
            return
        }

        if (!passphrase) {
            setErrors(
                t("keysGeneration.checkStep.noPassphrase", {
                    defaultValue: "Passphrase is required to open the key file",
                }),
            )
            return
        }

        const firstFile = files[0]

        const readFileContent = (file: File) => {
            return new Promise<string>((resolve, reject) => {
                const fileReader = new FileReader()
                fileReader.onload = () => resolve(fileReader.result as string)
                fileReader.onerror = error => reject(error)
                fileReader.readAsText(file)
            })
        }

        try {
            const fileContent = await readFileContent(firstFile)
            if (fileContent == null) {
                setErrors(t("keysGeneration.checkStep.noFileSelected"))
                return
            }

            let parsed: any
            try {
                parsed = JSON.parse(fileContent)
            } catch (e) {
                setErrors(
                    t("keysGeneration.checkStep.errorUploading", {
                        error: "invalid JSON key file",
                    }),
                )
                return
            }

            const trusteeName = authContext.trustee
            if (!trusteeName) {
                setErrors("Missing trustee identifier for current user")
                return
            }

            const KEY_COMMITMENT_ITERATIONS = 600000

            setUploading(true)
            const result = await wasmService.importKeyFile(
                parsed,
                passphrase,
                electionEvent.id,
                trusteeName,
                KEY_COMMITMENT_ITERATIONS,
            )
            setUploading(false)

            if (!result.isValid) {
                setErrors(
                    t("keysGeneration.checkStep.errorUploading", {
                        error: "commitment mismatch",
                    }),
                )
                return
            }

            setVerified(true)
        } catch (exception: any) {
            setUploading(false)
            setErrors(
                t("keysGeneration.checkStep.errorUploading", {
                    error: exception?.toString() ?? "unknown",
                }),
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
                    <Typography variant="body1">
                        <Trans
                            i18nKey="keysGeneration.checkStep.subtitle"
                            values={{name: authContext.username}}
                        />
                    </Typography>

                    <Typography variant="body2" sx={{mt: 2}}>
                        {t("keysGeneration.checkStep.passphraseHint", {
                            defaultValue:
                                "Enter the same passphrase you used when downloading your trustee key file.",
                        })}
                    </Typography>

                    <TextField
                        type="password"
                        label={t("keysGeneration.checkStep.passphraseLabel", {
                            defaultValue: "Passphrase",
                        })}
                        value={passphrase}
                        onChange={e => setPassphrase(e.target.value)}
                        fullWidth
                        margin="dense"
                        sx={{mt: 2}}
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
