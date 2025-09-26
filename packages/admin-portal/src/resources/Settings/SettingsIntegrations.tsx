// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useState} from "react"

import {useTranslation} from "react-i18next"
import {SimpleForm, useEditController, Toolbar, SaveButton, useNotify, TextInput} from "react-admin"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"

export const SettingsIntegrations: React.FC<void> = () => {
    const [tenantId] = useTenantStore()
    const {t, i18n} = useTranslation()
    const notify = useNotify()
    const authContext = useContext(AuthContext)

    const {record, save, isLoading} = useEditController({
        resource: "sequent_backend_tenant",
        id: tenantId,
        redirect: false,
        undoable: false,
    })

    const canEdit = authContext.isAuthorized(true, authContext.tenantId, [IPermissions.TENANT_WRITE, IPermissions.GOOGLE_MEET_API_TOKENS])
    const [gapiKey, setGapiKey] = useState<object | null>(null)
    const [gapiEmail, setGapiEmail] = useState<string>('')
    const [gapiKeyChanged, setGapiKeyChanged] = useState<boolean>(false)
    const [gapiEmailChanged, setGapiEmailChanged] = useState<boolean>(false)
    const [saveDisabled, setSaveDisabled] = useState<boolean>(true)

    useEffect(() => {
        console.log("useEffect gapiKey: ", saveDisabled)

        if (gapiKeyChanged || gapiEmailChanged) {
            setSaveDisabled(false)
        } else {
            setSaveDisabled(true)
        }
    }, [gapiKeyChanged, gapiEmailChanged])

    const handleGapiKeyChange = (
        event: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>
    ) => {
        console.log("handleGapiKeyChange")
        const inputValue = event.target.value
        try {
            const parsedGapiKey =
                !inputValue || inputValue.trim().length === 0
                    ? null
                    : JSON.parse(inputValue)

            if (typeof parsedGapiKey === 'object' && parsedGapiKey !== null) {
                setGapiKey(parsedGapiKey)
                setGapiKeyChanged(true)
                console.log("setGapiKey")
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
        console.log("handleGapiEmailChange")
        const inputValue = event.target.value
        setGapiEmail(inputValue)
        setGapiEmailChanged(true)
    }

    const onSave = async () => {
        const updatedSettings = { ...(record?.settings ?? {}) }
        
        // Only update fields that have been changed
        if (gapiKeyChanged) {
            updatedSettings.gapi_key = gapiKey
        }
        if (gapiEmailChanged) {
            updatedSettings.gapi_email = gapiEmail.trim() !== '' ? gapiEmail : undefined
        }
        
        save!({
            settings: updatedSettings,
        })
        // Clear the inputs and reset change flags after saving
        setGapiKey(null)
        setGapiEmail('')
        setGapiKeyChanged(false)
        setGapiEmailChanged(false)
    }

    if (isLoading) return null
    return (
        <SimpleForm
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
                source={"settings.gapi_key"}
                label={t("integrationsScreen.common.gapiKey")}
                onChange={handleGapiKeyChange}
            />
            <TextInput
                source={"settings.gapi_email"}
                label={t("integrationsScreen.common.gapiEmail")}
                onChange={handleGapiEmailChange}
            />
        </SimpleForm>
    )
}
