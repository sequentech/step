// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useRef, useState} from "react"
import {useTranslation} from "react-i18next"
import {useNotify} from "react-admin"
import {
    Alert,
    Autocomplete,
    Box,
    Button,
    Dialog,
    DialogActions,
    DialogContent,
    DialogTitle,
    FormControl,
    InputLabel,
    MenuItem,
    Select,
    TextField,
    Typography,
} from "@mui/material"
import UploadFileIcon from "@mui/icons-material/UploadFile"
import {CertificateCheckId, type IStaffCertificate} from "@/lib/signing/types"
import {
    SigningErrorCode,
    certificateFingerprint,
    linkTarget,
    personName,
    signingError,
    toPem,
} from "./signingSettings"
import {
    useScopeNames,
    usePeopleSearch,
    useRegisterCertificate,
    useWriteError,
    type IStaffPerson,
} from "./useSigningSettings"

const ALL_POSTS = "all"
export const CERTIFICATE_FILES = ".pem,.cer,.crt,.der"
const mono = {fontFamily: "monospace", fontSize: "0.75rem"}

/** Reads a chosen certificate file as PEM (PEM, CER or DER). */
export const readPem = async (file: File) => toPem(new Uint8Array(await file.arrayBuffer()))

/** A button that opens the file chooser of its hidden, named file input. */
export const FileButton: React.FC<{
    label: string
    accept: string
    onFile: (file: File) => void
}> = ({label, accept, onFile}) => {
    const input = useRef<HTMLInputElement>(null)
    return (
        <>
            <Button
                variant="outlined"
                startIcon={<UploadFileIcon />}
                onClick={() => input.current?.click()}
            >
                {label}
            </Button>
            <input
                ref={input}
                type="file"
                accept={accept}
                aria-label={label}
                hidden
                onChange={(event) => {
                    const file = event.target.files?.[0]
                    if (file) onFile(file)
                    event.target.value = ""
                }}
            />
        </>
    )
}

const fullName = (person: IStaffPerson) =>
    personName([person.first_name, person.last_name].filter(Boolean).join(" "), person.username)

export interface IRegisterCertificateDialogProps {
    open: boolean
    electionEventId: string
    /** The event's registrations: a refused one may belong to the same person's other account. */
    certificates: IStaffCertificate[]
    onClose: () => void
    onRegistered: () => void
}

/**
 * Registers a certificate to a person, for one Post or all. A certificate
 * already registered to another account can be linked to it as the same
 * person's second account (decided O1).
 */
export const RegisterCertificateDialog: React.FC<IRegisterCertificateDialogProps> = ({
    open,
    electionEventId,
    certificates,
    onClose,
    onRegistered,
}) => {
    const {t} = useTranslation()
    const notify = useNotify()
    const writeError = useWriteError()
    const {elections, postName} = useScopeNames(electionEventId)
    const [search, setSearch] = useState("")
    const {people, loading} = usePeopleSearch(search)
    const [person, setPerson] = useState<IStaffPerson | null>(null)
    const [post, setPost] = useState(ALL_POSTS)
    const [pem, setPem] = useState("")
    // The account holding the certificate, when the server refused it as someone else's.
    const [conflict, setConflict] = useState<{userId: string; name: string} | null>(null)
    const [register, {loading: saving}] = useRegisterCertificate()

    const close = () => {
        setSearch("")
        setPerson(null)
        setPost(ALL_POSTS)
        setPem("")
        setConflict(null)
        onClose()
    }

    /**
     * The account holding the certificate: the refusal names it; older answers
     * don't, so the event's registrations of the same certificate do.
     */
    const holderOf = async (named?: {userId: string; name: string | null}) => {
        const userId = named?.userId ?? linkTarget(certificates, await certificateFingerprint(pem))
        if (!userId) return null
        const row = certificates.find(({user_id}) => user_id === userId)
        const name = named?.name ?? (row ? personName(row.user_display_name, row.username) : userId)
        return {userId, name}
    }

    const run = async (linkedTo: string | null) => {
        if (!person) return
        try {
            await register({
                variables: {
                    election_event_id: electionEventId,
                    user_id: person.id,
                    election_id: post === ALL_POSTS ? null : post,
                    pem,
                    linked_to: linkedTo,
                },
            })
        } catch (error) {
            const refusal = signingError(error)
            const {code} = refusal
            const holder =
                code === SigningErrorCode.Refused &&
                refusal.check === CertificateCheckId.RegisteredToOther
                    ? await holderOf(refusal.holder)
                    : null
            if (holder && !linkedTo) {
                setConflict(holder)
                return
            }
            // For a registration, a conflict is the same certificate already registered to them.
            if (code === SigningErrorCode.Conflict) {
                notify(t("signing.certificates.alreadyRegistered"), {type: "error"})
                return
            }
            writeError(
                error,
                code === SigningErrorCode.Refused
                    ? "signing.certificates.registerRefused"
                    : "signing.certificates.registerError"
            )
            return
        }
        notify(t("signing.certificates.registerDone"), {type: "success"})
        close()
        onRegistered()
    }

    return (
        <Dialog open={open} onClose={close} fullWidth maxWidth="sm">
            <DialogTitle>{t("signing.certificates.register")}</DialogTitle>
            <DialogContent sx={{display: "grid", gap: 2, pt: "8px !important"}}>
                <Autocomplete
                    options={people}
                    loading={loading}
                    value={person}
                    onChange={(_event, value) => {
                        setPerson(value)
                        setConflict(null)
                    }}
                    inputValue={search}
                    onInputChange={(_event, value) => setSearch(value)}
                    getOptionLabel={(option) => `${fullName(option)} (${option.username})`}
                    isOptionEqualToValue={(option, value) => option.id === value.id}
                    filterOptions={(options) => options}
                    noOptionsText={t("signing.certificates.personSearchHelp")}
                    renderInput={(params) => (
                        <TextField {...params} required label={t("signing.certificates.person")} />
                    )}
                />
                <FormControl>
                    <InputLabel id="signing-register-post">
                        {t("signing.certificates.columns.post")}
                    </InputLabel>
                    <Select
                        labelId="signing-register-post"
                        label={t("signing.certificates.columns.post")}
                        value={post}
                        onChange={(event) => setPost(event.target.value)}
                    >
                        <MenuItem value={ALL_POSTS}>{t("signing.certificates.allPosts")}</MenuItem>
                        {elections.map((election) => (
                            <MenuItem key={election.id} value={election.id}>
                                {postName(election.id)}
                            </MenuItem>
                        ))}
                    </Select>
                </FormControl>
                <Box>
                    <FileButton
                        label={t("signing.certificates.chooseFile")}
                        accept={CERTIFICATE_FILES}
                        onFile={(file) =>
                            readPem(file).then(
                                (text) => {
                                    setPem(text)
                                    setConflict(null)
                                },
                                () => notify(t("signing.certificates.fileError"), {type: "error"})
                            )
                        }
                    />
                </Box>
                <TextField
                    required
                    multiline
                    minRows={4}
                    label={t("signing.certificates.pem")}
                    value={pem}
                    onChange={(event) => {
                        setPem(event.target.value)
                        setConflict(null)
                    }}
                    slotProps={{htmlInput: {style: mono}}}
                />
                {conflict && (
                    <Alert
                        severity="warning"
                        action={
                            <Button
                                color="inherit"
                                disabled={saving}
                                onClick={() => run(conflict.userId)}
                            >
                                {t("signing.certificates.linkAccount")}
                            </Button>
                        }
                    >
                        <Typography variant="body2">
                            {t("signing.certificates.registeredToOther", {name: conflict.name})}
                        </Typography>
                    </Alert>
                )}
            </DialogContent>
            <DialogActions>
                <Button onClick={close}>{t("common.label.cancel")}</Button>
                <Button
                    variant="contained"
                    disabled={!person || !pem.trim() || saving || !!conflict}
                    onClick={() => run(null)}
                >
                    {t("signing.certificates.registerSubmit")}
                </Button>
            </DialogActions>
        </Dialog>
    )
}
