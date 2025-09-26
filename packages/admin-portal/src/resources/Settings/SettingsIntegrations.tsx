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
    const [saveDisabled, setSaveDisabled] = useState<boolean>(true)

    useEffect(() => {
        console.log("useEffect gapiKey: ", saveDisabled)

        if (gapiKey !== null) {
            setSaveDisabled(false)
        } else {
            setSaveDisabled(true)
        }
    }, [gapiKey])

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
                console.log("setGapiKey")
            } else if (parsedGapiKey === null) {
                setGapiKey(null)
            } else {
                notify(t("integrationsScreen.errors.invalidGapiKey"), {type: "error"})
            }
        } catch (error) {
            notify(t("integrationsScreen.errors.invalidGapiKey"), {type: "error"})
        }
    }

    const onSave = async () => {
        save!({
            settings: {
                ...(record?.settings ?? {}),
                gapi_key: gapiKey,
            },
        })
        // Clear the input after saving
        setGapiKey(null)
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
        </SimpleForm>
    )
}
