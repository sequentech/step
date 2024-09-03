// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useState} from "react"

import styled from "@emotion/styled"
import {useTranslation} from "react-i18next"
import {SimpleForm, TextInput, useEditController, Toolbar, SaveButton} from "react-admin"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {AuthContext} from "@/providers/AuthContextProvider"
import {ITenantSettings} from "@sequentech/ui-core"
import {IPermissions} from "@/types/keycloak"

export const SettingsLookAndFeel: React.FC<void> = () => {
    const [tenantId] = useTenantStore()
    const {t, i18n} = useTranslation()
    const authContext = useContext(AuthContext)

    const {record, save, isLoading} = useEditController({
        resource: "sequent_backend_tenant",
        id: tenantId,
        redirect: false,
        undoable: false,
    })

    const canEdit = authContext.isAuthorized(true, authContext.tenantId, IPermissions.TENANT_WRITE)

    const [logoUrl, setLogoUrl] = useState<string | undefined>(
        (record?.settings as ITenantSettings | undefined)?.logo_url
    )

    const onSave = async () => {
        const newRecord = {...record}
        if (logoUrl === "") {
            newRecord.settings = {...newRecord.settings, logo_url: null}
        } else {
            newRecord.settings = {...newRecord.settings, logo_url: logoUrl}
        }
        console.log("onSave :>> ", newRecord?.settings?.logo_url)
        console.log("save :>> ")
        save!({
            settings: {
                ...((newRecord?.settings as ITenantSettings | undefined) ?? {}),
            },
        })
    }

    if (isLoading) return null

    return (
        <SimpleForm
            // defaultValues={{electionsOrder: sortedElections}}
            // validate={formValidator}
            // record={parsedValue}
            toolbar={
                <Toolbar>
                    {canEdit ? (
                        <SaveButton
                            onClick={() => {
                                onSave()
                            }}
                            type="button"
                        />
                    ) : null}
                </Toolbar>
            }
        >
            <TextInput
                resettable={true}
                source={"settings.logo_url"}
                label={t("electionTypeScreen.common.logoUrl")}
                onChange={(event) => setLogoUrl(event.target.value)}
            />
            <TextInput
                resettable={true}
                multiline={true}
                source={"settings.css"}
                label={t("electionTypeScreen.common.css")}
            />
        </SimpleForm>
    )
}
