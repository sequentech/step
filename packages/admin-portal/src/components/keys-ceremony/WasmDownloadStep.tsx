// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useApolloClient} from "@apollo/client"
import React, {useContext, useState} from "react"
import {FormControlLabel, FormGroup, Typography, Checkbox, TextField} from "@mui/material"
import ArrowForwardIosIcon from "@mui/icons-material/ArrowForwardIos"
import DownloadIcon from "@mui/icons-material/Download"
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
import {Dialog} from "@sequentech/ui-essentials"
import {useNotify} from "react-admin"

export interface WasmDownloadStepProps {
    electionEvent: Sequent_Backend_Election_Event
    currentCeremony: Sequent_Backend_Keys_Ceremony
    goNext: () => void
    goBack: () => void
}

export const WasmDownloadStep: React.FC<WasmDownloadStepProps> = ({
    electionEvent,
    currentCeremony: _currentCeremony,
    goNext,
    goBack,
}) => {
    const {t} = useTranslation()
    const authContext = useContext(AuthContext)
    const notify = useNotify()
    const apolloClient = useApolloClient()
    const wasmService = React.useMemo(
        () => new TrusteeWasmService(BraidWasm as unknown as IBraidWasmModule, apolloClient),
        [apolloClient]
    )

    const [downloaded, setDownloaded] = useState<boolean>(false)
    const [downloading, setDownloading] = useState<boolean>(false)
    const [openConfirmationModal, setOpenConfirmationModal] = useState(false)
    const [errors, setErrors] = useState<string | null>(null)
    const [checkboxState, setCheckboxState] = React.useState({
        firstCheckbox: false,
        secondCheckbox: false,
    })
    const {firstCheckbox, secondCheckbox} = checkboxState

    const [passphrase, setPassphrase] = useState<string>("")
    const [confirmPassphrase, setConfirmPassphrase] = useState<string>("")

    const handleCheckboxChange = (event: React.ChangeEvent<HTMLInputElement>) => {
        setCheckboxState({
            ...checkboxState,
            [event.target.name]: event.target.checked,
        })
    }

    const download = async () => {
        setErrors(null)
        setDownloaded(false)
        setDownloading(true)
        try {
            const trusteeName = authContext.trustee
            if (!trusteeName) {
                setErrors("Missing trustee identifier for current user")
                return
            }

            if (!passphrase || passphrase !== confirmPassphrase) {
                setErrors("Passphrase is required and must match confirmation")
                return
            }

            const KEY_COMMITMENT_ITERATIONS = 600000
            const FILE_KDF_ITERATIONS = 600000

            const generated = await wasmService.generateKeypair(
                electionEvent.id,
                trusteeName,
                KEY_COMMITMENT_ITERATIONS,
            )

            const keyFile = await wasmService.exportKeyFile(
                generated.key_id,
                electionEvent.id,
                trusteeName,
                generated.public_key_b64,
                passphrase,
                FILE_KDF_ITERATIONS,
            )

            const username = authContext.username || trusteeName
            const fileName = `trustee_key_${username}_${electionEvent.id}.json`

            const blob = new Blob([JSON.stringify(keyFile, null, 2)], {
                type: "application/json",
            })
            const blobUrl = window.URL.createObjectURL(blob)
            const tempLink = document.createElement("a")
            tempLink.href = blobUrl
            tempLink.setAttribute("download", fileName)
            tempLink.click()
            window.URL.revokeObjectURL(blobUrl)

            setDownloaded(true)
        } catch (exception: any) {
            setErrors(
                t("keysGeneration.downloadStep.errorDownloading", {
                    error: exception?.toString() ?? "unknown",
                })
            )
        } finally {
            setDownloading(false)
        }
    }

    return (
        <>
            <WizardStyles.ContentBox>
                <WizardStyles.StepHeader variant="h4">
                    {t("keysGeneration.downloadStep.title")}
                </WizardStyles.StepHeader>
                <WizardStyles.MainContent>
                    <Typography variant="body1">
                        <Trans
                            i18nKey="keysGeneration.downloadStep.subtitle"
                            values={{name: authContext.username}}
                        />
                    </Typography>

                    <Typography variant="body2" sx={{mt: 2}}>
                        {t("keysGeneration.downloadStep.choosePassphrase", {
                            defaultValue:
                                "Choose a passphrase that will be required to restore your trustee key file.",
                        })}
                    </Typography>

                    <FormGroup sx={{mt: 2}}>
                        <TextField
                            type="password"
                            label={t("keysGeneration.downloadStep.passphraseLabel", {
                                defaultValue: "Passphrase",
                            })}
                            value={passphrase}
                            onChange={e => setPassphrase(e.target.value)}
                            fullWidth
                            margin="dense"
                        />
                        <TextField
                            type="password"
                            label={t("keysGeneration.downloadStep.confirmPassphraseLabel", {
                                defaultValue: "Confirm passphrase",
                            })}
                            value={confirmPassphrase}
                            onChange={e => setConfirmPassphrase(e.target.value)}
                            fullWidth
                            margin="dense"
                        />
                    </FormGroup>

                    <WizardStyles.DownloadButton
                        color="primary"
                        onClick={download}
                        className="keys-download-download-button"
                    >
                        <DownloadIcon />
                        {t("keysGeneration.downloadStep.downloadButton")}
                    </WizardStyles.DownloadButton>

                    {downloading && <WizardStyles.DownloadProgress />}
                    {downloaded && !downloading && (
                        <Typography className="keys-download-success">
                            {t("keysGeneration.checkStep.downloaded")}
                        </Typography>
                    )}
                    {errors && !downloading && (
                        <Typography color="error" className="keys-download-error">
                            {errors}
                        </Typography>
                    )}
                </WizardStyles.MainContent>

                <WizardStyles.NavigationButtons>
                    <WizardStyles.BackButton
                        onClick={goBack}
                        startIcon={<ArrowBackIosIcon />}
                        className="keys-download-back-button"
                    >
                        {t("common.back")}
                    </WizardStyles.BackButton>
                    <WizardStyles.NextButton
                        color="primary"
                        variant="contained"
                        endIcon={<ArrowForwardIosIcon />}
                        disabled={!downloaded}
                        onClick={() => setOpenConfirmationModal(true)}
                        className="keys-download-next-button"
                    >
                        {t("common.next")}
                    </WizardStyles.NextButton>
                </WizardStyles.NavigationButtons>
            </WizardStyles.ContentBox>

            <Dialog
                title={t("keysGeneration.downloadStep.confirmdDialog.title")}
                cancelLabel={t("keysGeneration.downloadStep.confirmdDialog.cancel")}
                okLabel={t("keysGeneration.downloadStep.confirmdDialog.confirm")}
                open={openConfirmationModal}
                okEnabled={() => firstCheckbox && secondCheckbox}
                handleClose={result => {
                    if (result && firstCheckbox && secondCheckbox) {
                        setOpenConfirmationModal(false)
                        goNext()
                    } else if (result) {
                        notify(
                            t("keysGeneration.downloadStep.confirmdDialog.confirmError"),
                            {type: "error"}
                        )
                    } else {
                        setOpenConfirmationModal(false)
                        setCheckboxState({
                            firstCheckbox: false,
                            secondCheckbox: false,
                        })
                    }
                }}
            >
                <FormGroup>
                    <FormControlLabel
                        control={
                            <Checkbox
                                checked={firstCheckbox}
                                onChange={handleCheckboxChange}
                                name="firstCheckbox"
                                color="primary"
                                className="keys-download-first-checkbox"
                            />
                        }
                        label={t("keysGeneration.downloadStep.confirmdDialog.firstCopy")}
                    />
                    <FormControlLabel
                        control={
                            <Checkbox
                                checked={secondCheckbox}
                                onChange={handleCheckboxChange}
                                name="secondCheckbox"
                                color="primary"
                                className="keys-download-second-checkbox"
                            />
                        }
                        label={t("keysGeneration.downloadStep.confirmdDialog.secondCopy")}
                    />
                </FormGroup>
            </Dialog>
        </>
    )
}
