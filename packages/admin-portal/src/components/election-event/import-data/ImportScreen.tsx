// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    Box,
    styled,
    Button,
    TextField,
    InputLabel,
    Typography,
    IconButton,
    Menu,
    MenuItem,
} from "@mui/material"
import MoreHorizIcon from "@mui/icons-material/MoreHoriz"
import {DropFile, Dialog, ProblemList, type Problem} from "@sequentech/ui-essentials"
import {FormStyles} from "@/components/styles/FormStyles"
import React, {useEffect, useRef, memo, useState} from "react"
import {useTranslation} from "react-i18next"
import {GetUploadUrlMutation} from "@/gql/graphql"
import {GET_UPLOAD_URL} from "@/queries/GetUploadUrl"
import {useMutation, useApolloClient} from "@apollo/client"
import {SimpleForm, useNotify} from "react-admin"
import {EFileIntegrityPolicy} from "@sequentech/ui-core"
import {selectImportFiles} from "@/services/ImportFileSelection"
import {ImportRequestError} from "@/services/ImportRequestError"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {
    prepareComelecConfig,
    applyComelecConfig,
    ComelecConfigError,
    type ComelecConfigPlan,
} from "./comelecConfigImport"

const DOWNLOAD_URL_LIFETIME_MS = 1000

interface ImportScreenProps {
    doImport: (documentId: string, sha256: string, password?: string) => Promise<void>
    uploadCallback?: (documentId: string, password?: string, shaField?: string) => Promise<void>
    doCancel: () => void
    onSourceChange?: () => void
    errors: string | null
    /**
     * Why the file was refused, one named problem each, when the backend could
     * say. Shown instead of `errors`, which is the same thing as one English
     * paragraph.
     */
    problems?: Problem[] | null
    fileIntegrityPolicy?: EFileIntegrityPolicy
    allowHistoricalSource?: boolean
    disableImport?: boolean
    refresh?: string
}

const stageLabels = {
    "select": "Select a native file to import.",
    "local-ready":
        "Local validation passed. Create uploads the generated ZIP, validates it with the backend and requests creation.",
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
            onSourceChange,
            uploadCallback,
            doImport,
            disableImport,
            refresh,
            errors,
            problems,
            allowHistoricalSource,
        } = props
        const requiresSha = props.fileIntegrityPolicy === EFileIntegrityPolicy.REQUIRED_SHA256
        const {globalSettings} = React.useContext(SettingsContext)
        const configuredUrl = globalSettings.ELECTION_EVENT_IMPORT_CONVERTER_BASE_URL
        const converterUrl = typeof configuredUrl === "string" ? configuredUrl.trim() : undefined
        const historicalEnabled = !!allowHistoricalSource && !!converterUrl
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
        const [selectionError, setSelectionError] = useState("")
        const [downloadAnchor, setDownloadAnchor] = useState<HTMLElement | null>(null)
        const [downloadError, setDownloadError] = useState("")
        const [HistoricalImportPlugin, setHistoricalImportPlugin] =
            useState<typeof import("./HistoricalImportPlugin").HistoricalImportPlugin>()
        const [pluginLoadError, setPluginLoadError] = useState("")
        const [historicalFiles, setHistoricalFiles] = useState<File[]>([])
        const previousConverterUrl = useRef(converterUrl)
        const [stage, setStage] = useState<keyof typeof stageLabels>("select")
        // COMELEC two-file flow: the tenant-config preflight plan (no mutation) and the
        // apply-stage feedback shown in this same drawer before the event import.
        const apolloClient = useApolloClient()
        const [tenantId] = useTenantStore()
        const [configPlan, setConfigPlan] = useState<ComelecConfigPlan | null>(null)
        const [configError, setConfigError] = useState<string>("")
        const [configStageMsg, setConfigStageMsg] = useState<string>("")

        useEffect(() => {
            setShaField("")
            setDocumentId(null)
        }, [refresh])

        useEffect(() => {
            const changed = previousConverterUrl.current !== converterUrl
            previousConverterUrl.current = converterUrl
            if ((!changed && historicalEnabled) || (!historicalFiles.length && !generatedHash))
                return
            setHistoricalFiles([])
            setGeneratedHash(null)
            setTheFile(undefined)
            setShaField("")
            setDocumentId(null)
            setStage("select")
            setDownloadAnchor(null)
            setDownloadError("")
        }, [converterUrl, historicalEnabled, historicalFiles.length, generatedHash])

        useEffect(() => {
            if (!historicalEnabled || !historicalFiles.length || HistoricalImportPlugin) return
            let active = true
            setPluginLoadError("")
            void import("./HistoricalImportPlugin").then(
                (module) => {
                    if (active) setHistoricalImportPlugin(() => module.HistoricalImportPlugin)
                },
                () => {
                    if (active)
                        setPluginLoadError(
                            String(
                                t("electionEventScreen.import.pluginLoadError", {
                                    defaultValue:
                                        "Could not load the local converter. Reload the page and try again, or use a native ZIP. No files were uploaded.",
                                })
                            )
                        )
                }
            )
            return () => {
                active = false
            }
        }, [historicalEnabled, historicalFiles, HistoricalImportPlugin, t])

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
                return uploadedId
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
            if (!selected.length) return
            setDownloadAnchor(null)
            setDownloadError("")
            onSourceChange?.()
            setSelectionError("")
            setTheFile(undefined)
            if (
                allowHistoricalSource ||
                selected.some((file) => /\.(sql|xlsx)$/i.test(file.name))
            ) {
                const selection = selectImportFiles(selected, historicalFiles, historicalEnabled)
                if (selection.kind === "error") {
                    setHistoricalFiles([])
                    setStage("select")
                    setSelectionError(selection.message)
                    return
                }
                if (selection.kind === "historical") {
                    setStage("local")
                    setHistoricalFiles(selection.files)
                    return
                }
            } else if (selected.length !== 1) {
                setSelectionError("Drop one native file at a time.")
                return
            }
            setHistoricalFiles([])
            const theFile = selected[0]
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

        const validSha = /^[a-fA-F0-9]{64}$/.test(shaField)

        const submitImport = async (id = documentId) => {
            if (!id || (requiresSha && !validSha)) return
            setLoading(true)
            setStage("creating")
            try {
                await doImport(id, shaField, password)
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
            if (requiresSha && !validSha) return
            if (generatedHash && !historicalEnabled) return
            if (generatedHash && theFile && !documentId) {
                // COMELEC flow: apply the previewed tenant configuration (roles +
                // display + reused trustees), awaiting its terminal success, BEFORE
                // importing the event. The generic native path skips this.
                if (historicalEnabled) {
                    if (!configPlan) {
                        setConfigError(
                            "Prepare and review the tenant configuration before importing."
                        )
                        return
                    }
                    setLoading(true)
                    setStage("creating")
                    try {
                        await applyComelecConfig({
                            client: apolloClient,
                            tenantId: String(tenantId),
                            plan: configPlan,
                            putFile: uploadFile,
                            onStage: (update) => setConfigStageMsg(update.detail),
                        })
                    } catch (error) {
                        setConfigError(
                            error instanceof ComelecConfigError
                                ? `${error.message}${
                                      error.pendingTaskId
                                          ? ` (pending task ${error.pendingTaskId}; reconcile before retrying)`
                                          : ""
                                  }`
                                : "The tenant-configuration apply failed; the event was not imported."
                        )
                        setLoading(false)
                        setStage("creation-error")
                        return
                    }
                    setLoading(false)
                }
                const id = await uploadFileToS3(theFile, "", generatedHash)
                if (id) await submitImport(id)
                return
            }
            if (!shaField) {
                setShowShaDialog(true)
                return
            }
            await submitImport()
        }

        const isWorking = () => loading || isUploading
        const downloadConvertedZip = () => {
            setDownloadAnchor(null)
            setDownloadError("")
            if (!historicalEnabled || !generatedHash || !theFile || isWorking()) return
            let url: string | undefined
            let link: HTMLAnchorElement | undefined
            try {
                // Reuse the accepted File itself: download and upload share identical bytes/name/type.
                url = URL.createObjectURL(theFile)
                link = document.createElement("a")
                link.href = url
                link.download = theFile.name
                document.body.appendChild(link)
                link.click()
            } catch {
                setDownloadError(
                    String(
                        t("electionEventScreen.import.downloadError", {
                            defaultValue:
                                "Could not start the ZIP download. Try Download ZIP again. This action does not upload or import the package.",
                        })
                    )
                )
            } finally {
                link?.remove()
                if (url) {
                    const objectUrl = url
                    // Let the browser begin consuming the download before releasing its URL.
                    setTimeout(() => URL.revokeObjectURL(objectUrl), DOWNLOAD_URL_LIFETIME_MS)
                }
            }
        }

        return (
            <Box sx={{padding: "0"}}>
                <TextField
                    disabled={isWorking() || !!generatedHash}
                    required={requiresSha}
                    helperText={
                        requiresSha ? t("electionEventScreen.import.shaRequiredHelp") : undefined
                    }
                    error={requiresSha && shaField.length > 0 && !validSha}
                    label={String(t("electionEventScreen.import.sha"))}
                    size="small"
                    value={shaField}
                    onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                        setShaField(e.target.value)
                    }
                />

                <Box component="fieldset" disabled={isWorking()} sx={{border: 0, m: 0, p: 0}}>
                    <DropFile
                        multiple={historicalEnabled}
                        formatLabel={allowHistoricalSource ? "ZIP / JSON / EZIP" : undefined}
                        handleFiles={async (files) => handleFiles(files)}
                    />
                </Box>

                {selectionError && (
                    <Typography role="alert" color="error">
                        {selectionError}
                    </Typography>
                )}
                {historicalEnabled &&
                    historicalFiles.length > 0 &&
                    (HistoricalImportPlugin ? (
                        <HistoricalImportPlugin
                            baseUrl={converterUrl}
                            disabled={isWorking()}
                            droppedFiles={historicalFiles}
                            onPrepare={() => {
                                setDownloadAnchor(null)
                                setDownloadError("")
                                setStage("local")
                                setDocumentId(null)
                                setShaField("")
                                setGeneratedHash(null)
                                setTheFile(undefined)
                                setConfigPlan(null)
                                setConfigError("")
                                setConfigStageMsg("")
                            }}
                            onAccepted={async (file, hash) => {
                                setPassword("")
                                setTheFile(file)
                                setShaField(hash)
                                setGeneratedHash(hash)
                                setStage("local-ready")
                                // Preflight the tenant configuration (no mutation) so the
                                // operator previews the role/trustee/display changes in
                                // this drawer before the Import action applies them.
                                setConfigError("")
                                setConfigPlan(null)
                                setConfigStageMsg("")
                                try {
                                    const plan = await prepareComelecConfig(
                                        apolloClient,
                                        String(tenantId)
                                    )
                                    setConfigPlan(plan)
                                } catch (error) {
                                    setConfigError(
                                        error instanceof ComelecConfigError
                                            ? error.message
                                            : "The tenant-configuration preflight failed; no change was made."
                                    )
                                }
                            }}
                        />
                    ) : (
                        <Typography
                            role={pluginLoadError ? "alert" : "status"}
                            color={pluginLoadError ? "error" : undefined}
                        >
                            {pluginLoadError ||
                                String(
                                    t("electionEventScreen.import.loadingConverter", {
                                        defaultValue: "Loading local converter…",
                                    })
                                )}
                        </Typography>
                    ))}

                {historicalEnabled && configError && (
                    <Typography role="alert" color="error" sx={{mt: 1}}>
                        {configError}
                    </Typography>
                )}
                {historicalEnabled && configPlan && (
                    <Box component="details" sx={{mt: 1}} data-testid="config-preview">
                        <summary>
                            {String(
                                t("electionEventScreen.import.configPreview", {
                                    defaultValue:
                                        "Tenant configuration to apply (preview — nothing changed yet)",
                                })
                            )}
                        </summary>
                        <Typography variant="body2">
                            Realm display: {configPlan.preflight.realmDisplayName}
                            {configPlan.preflight.appliedTenantDisplayName
                                ? `; tenant display: ${configPlan.preflight.appliedTenantDisplayName}`
                                : ""}
                            . Roles: {configPlan.preflight.roles.length} across{" "}
                            {configPlan.preflight.groups.length} groups. Trustees reused:{" "}
                            {configPlan.reusedTrustees
                                .map((trustee) => trustee.name || trustee.id)
                                .join(", ") || "none"}
                            .
                        </Typography>
                        <Box component="ul" sx={{m: 0, pl: 3}}>
                            {configPlan.rolePreflight.changes
                                .filter(
                                    (change) =>
                                        change.additions.length || change.removals.length
                                )
                                .map((change) => (
                                    <li key={change.group}>
                                        {change.group}
                                        {change.exists ? "" : " (new)"}: +
                                        {change.additions.length} / -{change.removals.length}
                                        {change.removals.length
                                            ? ` (removing ${change.removals.join(", ")})`
                                            : ""}
                                    </li>
                                ))}
                        </Box>
                        {configStageMsg && (
                            <Typography variant="body2" sx={{mt: 0.5}}>
                                {configStageMsg}
                            </Typography>
                        )}
                    </Box>
                )}

                {stage !== "select" && (
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
                )}
                {downloadError && (
                    <Typography role="alert" color="error">
                        {downloadError}
                    </Typography>
                )}
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
                    {historicalEnabled && generatedHash && theFile && (
                        <>
                            <IconButton
                                aria-label={String(
                                    t("electionEventScreen.import.downloadOptions", {
                                        defaultValue: "Converted ZIP options",
                                    })
                                )}
                                aria-haspopup="menu"
                                aria-expanded={!!downloadAnchor}
                                disabled={isWorking()}
                                onClick={(event) => setDownloadAnchor(event.currentTarget)}
                            >
                                <MoreHorizIcon />
                            </IconButton>
                            <Menu
                                anchorEl={downloadAnchor}
                                open={!!downloadAnchor}
                                onClose={() => setDownloadAnchor(null)}
                            >
                                <MenuItem onClick={downloadConvertedZip} disabled={isWorking()}>
                                    {String(
                                        t("electionEventScreen.import.downloadZip", {
                                            defaultValue: "Download ZIP",
                                        })
                                    )}
                                </MenuItem>
                            </Menu>
                        </>
                    )}
                    <ImportStyles.ImportButton
                        sx={{marginLeft: generatedHash ? 1 : "auto"}}
                        disabled={
                            (!documentId && !generatedHash) ||
                            (!!generatedHash && !historicalEnabled) ||
                            isWorking() ||
                            disableImport ||
                            stage === "validation-error" ||
                            !!errors ||
                            problems?.some((p) => p.severity === "error") ||
                            showShaDialog ||
                            (requiresSha && !validSha)
                        }
                        onClick={onImportButtonClick}
                    >
                        {generatedHash
                            ? String(
                                  t("electionEventScreen.import.create", {defaultValue: "Create"})
                              )
                            : t("electionEventScreen.import.import")}
                    </ImportStyles.ImportButton>
                </Box>

                <Dialog
                    variant="warning"
                    open={!requiresSha && showShaDialog}
                    ok={String(t("electionEventScreen.import.shaDialog.ok"))}
                    cancel={String(t("electionEventScreen.import.shaDialog.cancel"))}
                    title={String(t("electionEventScreen.import.shaDialog.title"))}
                    handleClose={(result: boolean) => {
                        if (result && !requiresSha) {
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
