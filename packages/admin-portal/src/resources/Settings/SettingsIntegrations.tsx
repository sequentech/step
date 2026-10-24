// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useState} from "react"

import {useTranslation} from "react-i18next"
import {useMutation} from "@apollo/client"
import {SimpleForm, useEditController, Toolbar, SaveButton, useNotify, TextInput} from "react-admin"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {SetGoogleServiceAccountKeyMutation} from "@/gql/graphql"
import {SET_GOOGLE_SERVICE_ACCOUNT_KEY} from "@/queries/SetGoogleServiceAccountKey"

export const SettingsIntegrations: React.FC<void> = () => {
    const [tenantId] = useTenantStore()
    const {t, i18n} = useTranslation()
    const notify = useNotify()
    const authContext = useContext(AuthContext)

    const {record, save, isLoading, refetch} = useEditController({
        resource: "sequent_backend_tenant",
        id: tenantId,
        redirect: false,
        undoable: false,
    })

    const canEdit = authContext.isAuthorized(true, authContext.tenantId, [
        IPermissions.TENANT_WRITE,
    ])
    const [gapiKey, setGapiKey] = useState<object | null>(null)
    const [gapiEmail, setGapiEmail] = useState<string>("")
    const [gapiKeyChanged, setGapiKeyChanged] = useState<boolean>(false)
    const [gapiEmailChanged, setGapiEmailChanged] = useState<boolean>(false)
    const [saveDisabled, setSaveDisabled] = useState<boolean>(true)
    const [formVersion, setFormVersion] = useState<number>(0)
    const [setGoogleServiceAccountKey] = useMutation<SetGoogleServiceAccountKeyMutation>(
        SET_GOOGLE_SERVICE_ACCOUNT_KEY
    )

    useEffect(() => {
        if (gapiKeyChanged || gapiEmailChanged) {
            setSaveDisabled(false)
        } else {
            setSaveDisabled(true)
        }
    }, [gapiKeyChanged, gapiEmailChanged])

    const handleGapiKeyChange = (
        event: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>
    ) => {
        const inputValue = event.target.value
        try {
            const parsedGapiKey =
                !inputValue || inputValue.trim().length === 0 ? null : JSON.parse(inputValue)

            if (typeof parsedGapiKey === "object" && parsedGapiKey !== null) {
                setGapiKey(parsedGapiKey)
                setGapiKeyChanged(true)
            } else if (parsedGapiKey === null) {
                setGapiKey(null)
                setGapiKeyChanged(true)
            } else {
                notify(t("integrationsScreen.errors.invalidGapiKey"), {type: "error"})
            }
        } catch (error) {
            notify(t("integrationsScreen.errors.invalidGapiKey"), {type: "error"})
        }
    }

    const handleGapiEmailChange = (
        event: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>
    ) => {
        const inputValue = event.target.value
        setGapiEmail(inputValue)
        setGapiEmailChanged(true)
    }

    const onSave = async () => {
        const storeGapiKey = gapiKeyChanged && gapiKey !== null
        if (storeGapiKey) {
            try {
                await setGoogleServiceAccountKey({variables: {serviceAccountKey: gapiKey}})
            } catch (error) {
                notify(t("integrationsScreen.errors.saveGapiKey"), {type: "error"})
                return
            }
            notify(t("integrationsScreen.common.gapiKeySaved"), {type: "success"})
            setFormVersion((version) => version + 1)
        }

        if (gapiEmailChanged) {
            const updatedSettings = {...(record?.settings ?? {})}
            if (storeGapiKey) {
                delete updatedSettings.gapi_key
            }
            updatedSettings.gapi_email = gapiEmail.trim() !== "" ? gapiEmail : undefined
            save!({
                settings: updatedSettings,
            })
        } else if (storeGapiKey) {
            await refetch()
        }
        // Clear the inputs and reset change flags after saving
        setGapiKey(null)
        setGapiEmail("")
        setGapiKeyChanged(false)
        setGapiEmailChanged(false)
    }

    if (isLoading) return null
    return (
        <SimpleForm
            key={formVersion}
            toolbar={
                <Toolbar>
                    {canEdit ? (
                        <SaveButton
                            onClick={() => {
                                onSave()
                            }}
                            type="button"
                            disabled={saveDisabled}
                        />
                    ) : null}
                </Toolbar>
            }
        >
            <TextInput
                multiline={true}
                maxRows={6}
                source={"gapi_key_input"}
                label={String(t("integrationsScreen.common.gapiKey"))}
                helperText={String(t("integrationsScreen.common.gapiKeyHelper"))}
                onChange={handleGapiKeyChange}
            />
            <TextInput
                source={"settings.gapi_email"}
                label={String(t("integrationsScreen.common.gapiEmail"))}
                onChange={handleGapiEmailChange}
            />
        </SimpleForm>
    )
}
