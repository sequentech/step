import {Box, styled, Button, TextField, CircularProgress} from "@mui/material"
import {DropFile, Dialog} from "@sequentech/ui-essentials"
import {FormStyles} from "@/components/styles/FormStyles"
import React, {useEffect, memo, useRef} from "react"
import {useTranslation} from "react-i18next"

interface ImportScreenProps {
    doImport: (file: FileList | null, sha: string) => void
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

        const [shaField, setShaField] = React.useState<string>("")
        const [showShaDialog, setShowShaDialog] = React.useState<boolean>(false)
        const [fileImport, setFileImport] = React.useState<FileList | null>(null)

        const handleFiles = (files: FileList | null) => {
            setFileImport(files)
        }

        useEffect(() => {
            setShaField("")
            setFileImport(null)
        }, [refresh])

        const onImportButtonClick = () => {
            if (!shaField) {
                setShowShaDialog(true)
                return
            }

            doImport(fileImport, shaField)            
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

                <DropFile handleFiles={handleFiles} />

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
                    <ImportStyles.CancelButton
                        disabled={isLoading}
                        onClick={() => doCancel()}
                    >
                        {t("electionEventScreen.import.cancel")}
                    </ImportStyles.CancelButton>
                    <ImportStyles.ImportButton
                        disabled={!fileImport || isLoading}
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
                            doImport(fileImport, shaField)
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
