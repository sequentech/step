// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Box, styled, Button, TextField, InputLabel, Typography} from "@mui/material"
import {DropFile, Dialog, ProblemList, type Problem} from "@sequentech/ui-essentials"
import {FormStyles} from "@/components/styles/FormStyles"
import React, {useEffect, useRef, memo, useState} from "react"
import {useTranslation} from "react-i18next"
import {GetUploadUrlMutation} from "@/gql/graphql"
import {GET_UPLOAD_URL} from "@/queries/GetUploadUrl"
import {useMutation} from "@apollo/client"
import {SimpleForm, useNotify} from "react-admin"
import {HistoricalImportPlugin} from "./HistoricalImportPlugin"
import {ImportRequestError} from "@/services/ImportRequestError"
import {SettingsContext} from "@/providers/SettingsContextProvider"

interface ImportScreenProps {
    doImport: (documentId: string, sha256: string, password?: string) => Promise<void>
    uploadCallback?: (documentId: string, password?: string, shaField?: string) => Promise<void>
    doCancel: () => void
    errors: string | null
    /**
     * Why the file was refused, one named problem each, when the backend could
     * say. Shown instead of `errors`, which is the same thing as one English
     * paragraph.
     */
    problems?: Problem[] | null
    allowHistoricalSource?: boolean
    disableImport?: boolean
    refresh?: string
}

const stageLabels = {
    "select":
        "1. Select a native package, or both historical sources locally. Source selection does not upload files.",
    "local":
        "2. Local conversion and validation. No backend request has been made for these sources.",
    "uploading": "3. Upload transport: sending the accepted native package to the backend.",
    "validating": "3. Backend dry-run validation: checkOnly. The event has not been created.",
    "ready":
        "3. Backend dry-run passed. Final Import/Create submits a separate creation task and can still fail.",
    "uploaded": "File upload completed. Import has not started.",
    "creating": "4. Backend creation request in progress.",
    "submitted":
        "4. Backend creation task submitted. Follow the import task for final success or errors.",
    "transport-error":
        "Upload transport failed. No backend dry-run or creation was completed. Retry the upload.",
    "validation-transport-error":
        "Backend dry-run request failed. Validation was not confirmed; retry the upload and check the backend connection.",
    "validation-error":
        "Backend dry-run validation refused this package. Review the backend diagnostics below; no event was created.",
    "creation-transport-error":
        "Backend creation request failed. Its outcome is unconfirmed; check the import task and event list before retrying.",
    "creation-error":
        "Backend creation failed after dry-run validation. Review the import task and backend diagnostics below.",
} as const

export const ImportStyles = {
    CancelButton: styled(Button)`
        margin-right: auto;
        background-color: ${({theme}) => theme.palette.grey[100]};
        color: ${({theme}) => theme.palette.brandColor};
    `,

    ImportButton: styled(Button)`
        margin-left: auto;
    `,
}

const PasswordInputStyle = styled(FormStyles.PasswordInput)`
    flex: 1;
    margin: 0;
`

export const ImportScreenMemo: React.MemoExoticComponent<React.FC<ImportScreenProps>> = memo(
    (props: ImportScreenProps): React.JSX.Element => {
        const {
            doCancel,
            uploadCallback,
            doImport,
            disableImport,
            refresh,
            errors,
            problems,
            allowHistoricalSource,
        } = props
        const {globalSettings} = React.useContext(SettingsContext)
        const converterUrl = globalSettings.ELECTION_EVENT_IMPORT_CONVERTER_BASE_URL?.trim()
        const {t} = useTranslation()
        const notify = useNotify()
        const [loading, setLoading] = useState<boolean>(false)
        const [shaField, setShaField] = React.useState<string>("")
        const [showShaDialog, setShowShaDialog] = React.useState<boolean>(false)
        const [isUploading, setIsUploading] = React.useState<boolean>(false)
        const [documentId, setDocumentId] = React.useState<string | null>(null)
        const [getUploadUrl] = useMutation<GetUploadUrlMutation>(GET_UPLOAD_URL)
        const [passwordDialogOpen, setPasswordDialogOpen] = useState<boolean>(false)
        const passwordInputRef = useRef<HTMLInputElement>(null)
        const [password, setPassword] = useState<string>("")
        const [theFile, setTheFile] = useState<File | undefined>()
        const [generatedHash, setGeneratedHash] = useState<string | null>(null)
        const [historicalFiles, setHistoricalFiles] = useState<File[]>([])
        const [stage, setStage] = useState<keyof typeof stageLabels>("select")

        useEffect(() => {
            setShaField("")
            setDocumentId(null)
        }, [refresh])

        // Used to autofocus the password input field in the modal dialog. Just
        // using the autofocus property in PasswordInputStyle's inputProps
        // doesn't work
        useEffect(() => {
            if (passwordDialogOpen && passwordInputRef?.current) {
                passwordInputRef.current.focus()
            }
        }, [passwordDialogOpen, passwordInputRef.current])

        const uploadFile = async (url: string, file: File) => {
            const response = await fetch(url, {
                method: "PUT",
                headers: {
                    "Content-Type": file.type,
                },
                body: file,
            })
            if (!response.ok) {
                throw new Error("File upload failed")
            }
        }

        const uploadFileToS3 = async (
            theFile: File,
            uploadPassword: string,
            packageHash = shaField
        ) => {
            setIsUploading(true)
            setStage("uploading")
            setDocumentId(null)
            let uploadedId: string
            try {
                const {data} = await getUploadUrl({
                    variables: {
                        name: theFile.name,
                        media_type: theFile.name.endsWith(".ezip")
                            ? "application/ezip"
                            : theFile.type,
                        size: theFile.size,
                        is_public: false,
                    },
                })
                if (!data?.get_upload_url?.url) throw new Error("missing upload URL")
                await uploadFile(data.get_upload_url.url, theFile)
                uploadedId = data.get_upload_url.document_id
            } catch {
                setStage("transport-error")
                setIsUploading(false)
                notify(t("electionEventScreen.import.fileUploadError"), {type: "error"})
                return
            }
            try {
                if (uploadCallback) {
                    setStage("validating")
                    await uploadCallback(uploadedId, uploadPassword, packageHash)
                }
                setDocumentId(uploadedId)
                setStage(allowHistoricalSource && uploadCallback ? "ready" : "uploaded")
                notify(t("electionEventScreen.import.fileUploadSuccess"), {type: "success"})
            } catch (error) {
                setStage(
                    error instanceof ImportRequestError && error.stage === "transport"
                        ? "validation-transport-error"
                        : "validation-error"
                )
                notify(t("electionEventScreen.import.fileUploadError"), {type: "error"})
            } finally {
                setIsUploading(false)
            }
        }

        const handleFiles = async (files: FileList | null) => {
            if (loading || isUploading || passwordDialogOpen) return
            setDocumentId(null)
            if (generatedHash && shaField === generatedHash) setShaField("")
            setGeneratedHash(null)
            // https://fullstackdojo.medium.com/s3-upload-with-presigned-url-react-and-nodejs-b77f348d54cc
            setPassword("")
            const selected = Array.from(files ?? [])
            if (selected.some((file) => /\.(sql|xlsx)$/i.test(file.name))) {
                if (!allowHistoricalSource || !converterUrl) {
                    notify("Historical import converter is not configured; choose a native ZIP.", {
                        type: "warning",
                    })
                    return
                }
                if (selected.some((file) => !/\.(sql|xlsx)$/i.test(file.name))) {
                    notify(
                        String(
                            t("electionEventScreen.import.historical.separateSources", {
                                defaultValue:
                                    "Choose a native archive or historical sources separately.",
                            })
                        ),
                        {type: "warning"}
                    )
                    return
                }
                if (
                    selected.filter((file) => /\.sql$/i.test(file.name)).length > 1 ||
                    selected.filter((file) => /\.xlsx$/i.test(file.name)).length > 1
                ) {
                    notify(
                        String(
                            t("electionEventScreen.import.historical.oneEach", {
                                defaultValue: "Choose one SQL dump and one workbook.",
                            })
                        ),
                        {type: "warning"}
                    )
                    return
                }
                setStage("local")
                setHistoricalFiles(selected)
                return
            }
            const theFile = files?.[0]
            setTheFile(theFile)
            const isEncrypted = theFile?.name.endsWith(".ezip") || false
            if (isEncrypted) {
                setPasswordDialogOpen(true)
                return
            }

            if (theFile) {
                await uploadFileToS3(theFile, "")
            } else {
                setIsUploading(false)
                notify(t("electionEventScreen.import.fileUploadError"), {type: "error"})
            }
        }

        const handlePasswordSubmit = async (value: boolean) => {
            setPasswordDialogOpen(false)
            if (!theFile || !value) {
                return
            }
            await uploadFileToS3(theFile, password)
        }

        const submitImport = async () => {
            if (!documentId) return
            setLoading(true)
            setStage("creating")
            try {
                await doImport(documentId, shaField, password)
                setStage("submitted")
            } catch (error) {
                setStage(
                    error instanceof ImportRequestError && error.stage === "transport"
                        ? "creation-transport-error"
                        : "creation-error"
                )
            } finally {
                setLoading(false)
            }
        }
        const onImportButtonClick = async () => {
            if (!shaField) {
                setShowShaDialog(true)
                return
            }
            await submitImport()
        }

        const isWorking = () => loading || isUploading

        return (
            <Box sx={{padding: "0"}}>
                <TextField
                    disabled={isWorking()}
                    label={String(t("electionEventScreen.import.sha"))}
                    size="small"
                    value={shaField}
                    onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                        setShaField(e.target.value)
                    }
                />

                <Box component="fieldset" disabled={isWorking()} sx={{border: 0, m: 0, p: 0}}>
                    <DropFile handleFiles={async (files) => handleFiles(files)} />
                </Box>

                {allowHistoricalSource && (
                    <HistoricalImportPlugin
                        baseUrl={converterUrl}
                        disabled={isWorking()}
                        droppedFiles={historicalFiles}
                        onPrepare={() => {
                            setStage("local")
                            setDocumentId(null)
                            setShaField("")
                        }}
                        onAccepted={async (file, hash) => {
                            setPassword("")
                            setTheFile(file)
                            setShaField(hash)
                            setGeneratedHash(hash)
                            await uploadFileToS3(file, "", hash)
                        }}
                    />
                )}

                <Typography
                    role="status"
                    data-testid="import-stage"
                    color={stage.endsWith("error") ? "error" : undefined}
                    sx={{mt: 2}}
                >
                    {String(
                        t(`electionEventScreen.import.stage.${stage}`, {
                            defaultValue: stageLabels[stage],
                        })
                    )}
                </Typography>
                <FormStyles.StatusBox>
                    {isWorking() ? <FormStyles.ShowProgress /> : null}
                    {problems && problems.length > 0 ? (
                        <Box sx={{width: "100%"}} data-testid="import-problems">
                            <Typography variant="body2">
                                {String(
                                    t("electionEventScreen.import.backendDiagnostics", {
                                        defaultValue:
                                            "Backend diagnostics from the latest validation or creation request",
                                    })
                                )}
                            </Typography>
                            <ProblemList report={{problems}} />
                        </Box>
                    ) : errors ? (
                        <FormStyles.ErrorMessage variant="body2">{errors}</FormStyles.ErrorMessage>
                    ) : null}
                </FormStyles.StatusBox>

                <Box
                    sx={{
                        display: "flex",
                        flexDirection: "row",
                        justifyContent: "space-between",
                        alignItems: "center",
                        marginTop: "16px",
                    }}
                >
                    <ImportStyles.CancelButton disabled={isWorking()} onClick={() => doCancel()}>
                        {t("electionEventScreen.import.cancel")}
                    </ImportStyles.CancelButton>
                    <ImportStyles.ImportButton
                        disabled={
                            !documentId ||
                            isWorking() ||
                            disableImport ||
                            !!errors ||
                            problems?.some((p) => p.severity === "error") ||
                            showShaDialog
                        }
                        onClick={onImportButtonClick}
                    >
                        {t("electionEventScreen.import.import")}
                    </ImportStyles.ImportButton>
                </Box>

                <Dialog
                    variant="warning"
                    open={showShaDialog}
                    ok={String(t("electionEventScreen.import.shaDialog.ok"))}
                    cancel={String(t("electionEventScreen.import.shaDialog.cancel"))}
                    title={String(t("electionEventScreen.import.shaDialog.title"))}
                    handleClose={(result: boolean) => {
                        if (result) {
                            void submitImport()
                        }

                        setShowShaDialog(false)
                    }}
                >
                    {t("electionEventScreen.import.shaDialog.description")}
                </Dialog>

                <Dialog
                    open={passwordDialogOpen}
                    handleClose={handlePasswordSubmit}
                    title={String(t("electionEventScreen.import.passwordDialog.title"))}
                    ok={"Ok"}
                    variant="info"
                >
                    <>
                        <SimpleForm toolbar={false} component={Box}>
                            <Box>
                                <InputLabel>
                                    {t("electionEventScreen.import.passwordDialog.description")}:
                                </InputLabel>
                                <PasswordInputStyle
                                    label={false}
                                    source="password"
                                    helperText={false}
                                    onChange={(e) => setPassword(e.target.value)}
                                    inputProps={{
                                        "ref": passwordInputRef,
                                        "aria-label": t(
                                            "electionEventScreen.import.passwordDialog.label"
                                        ),
                                    }}
                                />
                            </Box>
                        </SimpleForm>
                    </>
                </Dialog>
            </Box>
        )
    }
)

ImportScreenMemo.displayName = "ImportScreen"

export const ImportScreen = ImportScreenMemo
