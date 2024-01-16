import {Box, styled, Button, TextField, CircularProgress} from "@mui/material"
import {DropFile, Dialog} from "@sequentech/ui-essentials"
import {FormStyles} from "@/components/styles/FormStyles"
import React, {useEffect, memo, useRef} from "react"
import {useTranslation} from "react-i18next"
import {GetUploadUrlMutation} from "@/gql/graphql"
import {GET_UPLOAD_URL} from "@/queries/GetUploadUrl"
import {useMutation} from "@apollo/client"
import {useNotify} from "react-admin"

interface ImportScreenProps {
    doImport: (documentId: string, sha256: string) => void
    doCancel: () => void
    isLoading: boolean
    errors: String | null
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

export const ImportScreenMemo: React.MemoExoticComponent<React.FC<ImportScreenProps>> = memo(
    (props: ImportScreenProps): React.JSX.Element => {
        const {doCancel, doImport, isLoading, refresh, errors} = props

        const {t} = useTranslation()
        const notify = useNotify()
        const [shaField, setShaField] = React.useState<string>("")
        const [showShaDialog, setShowShaDialog] = React.useState<boolean>(false)
        const [documentId, setDocumentId] = React.useState<string | null>(null)
        const [getUploadUrl] = useMutation<GetUploadUrlMutation>(GET_UPLOAD_URL)

        const handleFiles = async (files: FileList | null) => {
            // https://fullstackdojo.medium.com/s3-upload-with-presigned-url-react-and-nodejs-b77f348d54cc

            const theFile = files?.[0]

            if (theFile) {
                // Get the Upload URL
                let {data, errors} = await getUploadUrl({
                    variables: {
                        name: theFile.name,
                        media_type: theFile.type,
                        size: theFile.size,
                        is_public: false,
                    },
                })

                try {
                    if (!data?.get_upload_url?.url) {
                        notify(t("electionEventScreen.import.fileUploadError"), {type: "error"})
                        return
                    }
                    // Actually upload the CSV file
                    await fetch(data.get_upload_url.url, {
                        method: "PUT",
                        headers: {
                            "Content-Type": "text/csv",
                        },
                        body: theFile,
                    })
                    notify(t("electionEventScreen.import.fileUploadSuccess"), {type: "success"})
                    setDocumentId(data.get_upload_url.document_id)
                } catch (_error) {
                    notify(t("electionEventScreen.import.fileUploadError"), {type: "error"})
                }
            }
        }

        useEffect(() => {
            setShaField("")
            setDocumentId(null)
        }, [refresh])

        const onImportButtonClick = () => {
            if (!shaField) {
                setShowShaDialog(true)
                return
            }

            doImport(documentId as string, shaField)
        }

        return (
            <Box sx={{padding: "16px"}}>
                <TextField
                    label={t("electionEventScreen.import.sha")}
                    size="small"
                    value={shaField}
                    onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                        setShaField(e.target.value)
                    }
                />

                <DropFile handleFiles={async (files) => handleFiles(files)} />

                <FormStyles.StatusBox>
                    {isLoading ? <FormStyles.ShowProgress /> : null}
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
                    <ImportStyles.CancelButton disabled={isLoading} onClick={() => doCancel()}>
                        {t("electionEventScreen.import.cancel")}
                    </ImportStyles.CancelButton>
                    <ImportStyles.ImportButton
                        disabled={!documentId || isLoading}
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
                            doImport(documentId as string, shaField)
                        }
                        setShowShaDialog(false)
                    }}
                >
                    {t("electionEventScreen.import.shaDialog.description")}
                </Dialog>
            </Box>
        )
    }
)

ImportScreenMemo.displayName = "ImportScreen"

export const ImportScreen = ImportScreenMemo
