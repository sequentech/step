// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext} from "react"
import {useQuery} from "@apollo/client"
import {CircularProgress, Typography} from "@mui/material"
import {Dialog} from "@sequentech/ui-essentials"
import {useTranslation} from "react-i18next"
import {GetDocumentPasswordQuery} from "@/gql/graphql"
import {GET_DOCUMENT_PASSWORD} from "@/queries/DocumentPassword"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {IPermissions} from "@/types/keycloak"
import {DecryptHelp, PasswordDialog} from "@/components/election-event/export-data/PasswordDialog"

export const reportDecryptionCommand =
    "openssl enc -d -aes-256-cbc -in <encrypted_file> -out <decrypted_file> -pass pass:<password> -md md5"

export interface ReportDocumentAccess {
    password_secret_id?: string
    voter_secret_attributes?: boolean
}

/** Also supports older encrypted reports which have no saved document password. */
export const ReportPasswordDialog = ({
    documentId,
    access,
    onClose,
}: {
    documentId: string
    access?: ReportDocumentAccess
    onClose: () => void
}) => {
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const auth = useContext(AuthContext)
    const allowed =
        auth.isAuthorized(true, tenantId, IPermissions.DOCUMENT_PASSWORD_READ) &&
        auth.isAuthorized(true, tenantId, IPermissions.DOCUMENT_DOWNLOAD) &&
        (!access?.voter_secret_attributes ||
            auth.isAuthorized(true, tenantId, IPermissions.VOTER_SECRET_ATTRIBUTE_READ))
    const {data, loading, error} = useQuery<GetDocumentPasswordQuery>(GET_DOCUMENT_PASSWORD, {
        variables: {documentId},
        skip: !allowed || !access?.password_secret_id,
        fetchPolicy: "no-cache",
        context: {headers: {"x-hasura-role": IPermissions.DOCUMENT_PASSWORD_READ}},
    })
    const password = allowed ? data?.get_document_password?.password : undefined
    if (password) {
        return (
            <PasswordDialog password={password} onClose={onClose}>
                <DecryptHelp decryptionCommand={reportDecryptionCommand} />
            </PasswordDialog>
        )
    }
    return (
        <Dialog
            variant="info"
            open={true}
            handleClose={onClose}
            title={String(t("reportsScreen.messages.decryptFileTitle"))}
            ok="Ok"
        >
            {loading && allowed ? <CircularProgress /> : null}
            {error || (access?.password_secret_id && !allowed) ? (
                <Typography color="error">
                    {t("tasksScreen.documentAccess.passwordError")}
                </Typography>
            ) : null}
            <DecryptHelp decryptionCommand={reportDecryptionCommand} />
        </Dialog>
    )
}
