// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Box, styled, Button, TextField, InputLabel} from "@mui/material"
import {DropFile, Dialog} from "@sequentech/ui-essentials"
import {FormStyles} from "@/components/styles/FormStyles"
import React, {useEffect, useRef, memo, useState} from "react"
import {useTranslation} from "react-i18next"
import {GetUploadUrlMutation} from "@/gql/graphql"
import {GET_UPLOAD_URL} from "@/queries/GetUploadUrl"
import {useMutation} from "@apollo/client"
import {SimpleForm, useNotify} from "react-admin"

interface ImportScreenProps {
    doImport: (documentId: string, sha256: string, password?: string) => Promise<void>
    uploadCallback?: (documentId: string, password?: string, shaField?: string) => Promise<void>
    doCancel: () => void
    errors: string | null
    disableImport?: boolean
    refresh?: string
}

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
        const {doCancel, uploadCallback, doImport, disableImport, refresh, errors} = props
        const {t} = useTranslation()
        const notify = useNotify()
        const [loading, setLoading] = useState<boolean>(false)
        const [shaField, setShaField] = React.useState<string>("")
        const [showShaDialog, setShowShaDialog] = React.useState<boolean>(false)
        const [isUploading, setIsUploading] = React.useState<boolean>(false)
        const [documentId, setDocumentId] = React.useState<string | null>(null)
        const [getUploadUrl] = useMutation<GetUploadUrlMutation>(GET_UPLOAD_URL)
        const [isEncrypted, setIsEncrypted] = useState<boolean>(false)
        const [passwordDialogOpen, setPasswordDialogOpen] = useState<boolean>(false)
        const passwordInputRef = useRef<HTMLInputElement>(null)
        const [password, setPassword] = useState<string>("")
        const [theFile, setTheFile] = useState<File | undefined>()

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
            await fetch(url, {
                method: "PUT",
                headers: {
                    "Content-Type": file.type,
                },
                body: file,
            })
            setIsUploading(false)
        }

        const uploadFileToS3 = async (theFile: File) => {
            try {
                // Get the Upload URL
                let {data} = await getUploadUrl({
                    variables: {
                        name: theFile.name,
                        media_type: isEncrypted ? "application/ezip" : theFile.type,
                        size: theFile.size,
                        is_public: false,
                    },
                })

                if (!data?.get_upload_url?.url) {
                    notify(t("electionEventScreen.import.fileUploadError"), {type: "error"})
                    return
                }

                await uploadFile(data.get_upload_url.url, theFile)
                setDocumentId(data.get_upload_url.document_id)
                if (uploadCallback) {
                    console.log("uploadCallback call")
                    await uploadCallback?.(data.get_upload_url.document_id, password, shaField)
                }
                notify(t("electionEventScreen.import.fileUploadSuccess"), {type: "success"})
            } catch (_error) {
                setIsUploading(false)
                notify(t("electionEventScreen.import.fileUploadError"), {type: "error"})
            }
        }

        const handleFiles = async (files: FileList | null) => {
            // https://fullstackdojo.medium.com/s3-upload-with-presigned-url-react-and-nodejs-b77f348d54cc
            setPassword("")
            const theFile = files?.[0]
            setTheFile(theFile)
            const isEncrypted = theFile?.name.endsWith(".ezip") || false
            setIsEncrypted(isEncrypted)
            if (isEncrypted) {
                setPasswordDialogOpen(true)
                return
            }

            if (theFile) {
                setIsUploading(true)
                await uploadFileToS3(theFile)
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
            await uploadFileToS3(theFile)
        }

        const onImportButtonClick = async () => {
            if (!shaField) {
                setShowShaDialog(true)
                return
            }

            setLoading(true)
            await doImport(documentId as string, shaField, password)
            setLoading(false)
        }

        const isWorking = () => loading || isUploading

        return (
            <Box sx={{padding: "0"}}>
                <TextField
                    disabled={isWorking()}
                    label={t("electionEventScreen.import.sha")}
                    size="small"
                    value={shaField}
                    onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                        setShaField(e.target.value)
                    }
                />

                <DropFile handleFiles={async (files) => handleFiles(files)} />

                <FormStyles.StatusBox>
                    {isWorking() ? <FormStyles.ShowProgress /> : null}
                    {errors ? (
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
                        disabled={!documentId || isWorking() || disableImport || showShaDialog}
                        onClick={onImportButtonClick}
                    >
                        {t("electionEventScreen.import.import")}
                    </ImportStyles.ImportButton>
                </Box>

                <Dialog
                    variant="warning"
                    open={showShaDialog}
                    ok={t("electionEventScreen.import.shaDialog.ok")}
                    cancel={t("electionEventScreen.import.shaDialog.cancel")}
                    title={t("electionEventScreen.import.shaDialog.title")}
                    handleClose={(result: boolean) => {
                        if (result) {
                            doImport(documentId as string, shaField, password)
                        }

                        setShowShaDialog(false)
                    }}
                >
                    {t("electionEventScreen.import.shaDialog.description")}
                </Dialog>

                <Dialog
                    open={passwordDialogOpen}
                    handleClose={handlePasswordSubmit}
                    title={t("electionEventScreen.import.passwordDialog.title")}
                    ok={"Ok"}
                    variant="info"
                >
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
                                    ref: passwordInputRef,
                                }}
                            />
                        </Box>
                    </SimpleForm>
                </Dialog>
            </Box>
        )
    }
)

ImportScreenMemo.displayName = "ImportScreen"

export const ImportScreen = ImportScreenMemo
