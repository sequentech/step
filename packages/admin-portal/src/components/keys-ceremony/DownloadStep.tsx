// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useMutation} from "@apollo/client"
import React, {useContext, useState} from "react"
import {FormControlLabel, FormGroup, Typography, Checkbox} from "@mui/material"
import ArrowForwardIosIcon from "@mui/icons-material/ArrowForwardIos"
import DownloadIcon from "@mui/icons-material/Download"
import ArrowBackIosIcon from "@mui/icons-material/ArrowBackIos"
import {Trans, useTranslation} from "react-i18next"

import {
    GetPrivateKeyMutation,
    Sequent_Backend_Election_Event,
    Sequent_Backend_Keys_Ceremony,
} from "@/gql/graphql"
import {AuthContext} from "@/providers/AuthContextProvider"
import {WizardStyles} from "@/components/styles/WizardStyles"
import {GET_PRIVATE_KEY} from "@/queries/GetPrivateKey"
import {Dialog} from "@sequentech/ui-essentials"
import {useNotify} from "react-admin"

export interface DownloadStepProps {
    electionEvent: Sequent_Backend_Election_Event
    currentCeremony: Sequent_Backend_Keys_Ceremony
    goNext: () => void
    goBack: () => void
}

export const DownloadStep: React.FC<DownloadStepProps> = ({
    electionEvent,
    currentCeremony,
    goNext,
    goBack,
}) => {
    const {t} = useTranslation()
    const authContext = useContext(AuthContext)
    const [downloaded, setDownloaded] = useState<boolean>(false)
    const [downloading, setDownloading] = useState<boolean>(false)
    const [openConfirmationModal, setOpenConfirmationModal] = useState(false)
    const [errors, setErrors] = useState<String | null>(null)
    const notify = useNotify()
    const [checkboxState, setCheckboxState] = React.useState({
        firstCheckbox: false,
        secondCheckbox: false,
    })
    const handleCheckboxChange = (event: React.ChangeEvent<HTMLInputElement>) => {
        console.log("aa changed", event.target.checked)

        setCheckboxState({
            ...checkboxState,
            [event.target.name]: event.target.checked,
        })
    }
    const {firstCheckbox, secondCheckbox} = checkboxState

    const [getPrivateKeysMutation] = useMutation<GetPrivateKeyMutation>(GET_PRIVATE_KEY)
    const download = async () => {
        setErrors(null)
        setDownloaded(false)
        setDownloading(true)
        try {
            const {data, errors} = await getPrivateKeysMutation({
                variables: {
                    electionEventId: electionEvent.id,
                    keysCeremonyId: currentCeremony.id,
                },
            })
            setDownloading(false)
            if (errors) {
                setErrors(
                    t("keysGeneration.downloadStep.errorDownloading", {error: errors.toString()})
                )
                return null
            } else {
                const privateKey = data?.get_private_key?.private_key_base64
                if (!privateKey) {
                    setErrors(t("keysGeneration.downloadStep.errorEmptyKey"))
                    return
                }
                const blob = new Blob([privateKey], {type: "text/plain"})
                const blobUrl = window.URL.createObjectURL(blob)
                const username = authContext.username
                const electionName = electionEvent.alias || electionEvent.name
                const fileName = `encrypted_private_key_trustee_${username}_${electionName}.txt`
                var tempLink = document.createElement("a")
                tempLink.href = blobUrl
                tempLink.setAttribute("download", fileName)
                tempLink.click()
                setDownloaded(true)
            }
        } catch (exception: any) {
            setDownloading(false)
            setErrors(
                t("keysGeneration.downloadStep.errorDownloading", {error: exception.toString()})
            )
            return null
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
                        ></Trans>
                    </Typography>
                    <WizardStyles.DownloadButton
                        color="primary"
                        onClick={download}
                        className="keys-download-download-button"
                    >
                        <DownloadIcon />
                        {t("keysGeneration.downloadStep.downloadButton")}
                    </WizardStyles.DownloadButton>
                    <WizardStyles.StatusBox>
                        {downloading ? <WizardStyles.DownloadProgress /> : null}
                        {downloaded ? (
                            <WizardStyles.SucessMessage
                                variant="body1"
                                className="keys-download-success"
                            >
                                {t("keysGeneration.checkStep.downloaded")}
                            </WizardStyles.SucessMessage>
                        ) : null}
                        {errors ? (
                            <WizardStyles.ErrorMessage
                                variant="body2"
                                className="keys-download-error"
                            >
                                {errors}
                            </WizardStyles.ErrorMessage>
                        ) : null}
                    </WizardStyles.StatusBox>
                </WizardStyles.MainContent>
            </WizardStyles.ContentBox>

            <WizardStyles.Toolbar>
                <WizardStyles.BackButton
                    color="info"
                    onClick={goBack}
                    className="keys-download-back-button"
                >
                    <ArrowBackIosIcon />
                    {t("common.label.back")}
                </WizardStyles.BackButton>
                <WizardStyles.NextButton
                    disabled={!downloaded}
                    color="info"
                    onClick={() => setOpenConfirmationModal(true)}
                    className="keys-download-next-button"
                >
                    <ArrowForwardIosIcon />
                    {t("common.label.next")}
                </WizardStyles.NextButton>
            </WizardStyles.Toolbar>
            <Dialog
                variant="info"
                open={openConfirmationModal}
                ok={String(t("keysGeneration.downloadStep.confirmdDialog.ok"))}
                cancel={String(t("keysGeneration.downloadStep.confirmdDialog.cancel"))}
                title={String(t("keysGeneration.downloadStep.confirmdDialog.title"))}
                okEnabled={() => firstCheckbox && secondCheckbox}
                handleClose={(result: boolean) => {
                    if (result) {
                        if (firstCheckbox && secondCheckbox) {
                            goNext()
                            setOpenConfirmationModal(false)
                        } else {
                            notify(t("keysGeneration.downloadStep.confirmdDialog.confirmError"), {
                                type: "error",
                            })
                        }
                    } else {
                        setCheckboxState({
                            firstCheckbox: false,
                            secondCheckbox: false,
                        })
                        setOpenConfirmationModal(false)
                    }
                }}
            >
                <Typography variant="body1">
                    {t("keysGeneration.downloadStep.confirmdDialog.description")}
                </Typography>
                <FormGroup>
                    <FormControlLabel
                        control={
                            <Checkbox
                                key="firstCheckbox"
                                checked={firstCheckbox}
                                onChange={handleCheckboxChange}
                                name="firstCheckbox"
                                className="keys-download-first-checkbox"
                            />
                        }
                        label={String(t("keysGeneration.downloadStep.confirmdDialog.firstCopy"))}
                    />
                    <FormControlLabel
                        control={
                            <Checkbox
                                key="secondCheckbox"
                                checked={secondCheckbox}
                                onChange={handleCheckboxChange}
                                name="secondCheckbox"
                                className="keys-download-second-checkbox"
                            />
                        }
                        label={String(t("keysGeneration.downloadStep.confirmdDialog.secondCopy"))}
                    />
                </FormGroup>
            </Dialog>
        </>
    )
}
