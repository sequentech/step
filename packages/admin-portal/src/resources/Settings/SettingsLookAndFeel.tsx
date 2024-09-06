// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useState} from "react"

import {useTranslation} from "react-i18next"
import {SimpleForm, TextInput, useEditController, Toolbar, SaveButton} from "react-admin"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {AuthContext} from "@/providers/AuthContextProvider"
import {ITenantTheme} from "@sequentech/ui-core"
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
        (record?.annotations as ITenantTheme | undefined)?.logo_url
    )

    const [cssContent, setCssContent] = useState<string | undefined>(
        (record?.annotations as ITenantTheme | undefined)?.css
    )

    const onSave = async () => {
        const logoUrlToSave = logoUrl === "" ? null : logoUrl
        const cssContentToSave = cssContent === "" ? null : cssContent

        console.log("onSave logoUrl: ", logoUrlToSave)
        console.log("onSave cssContent: ", cssContentToSave)
        save!({
            annotations: {
                ...((record?.annotations as ITenantTheme | undefined) ?? {}),
                logo_url: logoUrlToSave,
                css: cssContentToSave,
            },
        })
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
                        />
                    ) : null}
                </Toolbar>
            }
        >
            <TextInput
                resettable={true}
                source={"annotations.logo_url"}
                defaultValue={logoUrl}
                label={t("electionTypeScreen.common.logoUrl")}
                onBlur={(event) => setLogoUrl(event.target.value)}
            />
            <TextInput
                resettable={true}
                multiline={true}
                source={"annotations.css"}
                defaultValue={cssContent}
                label={t("electionTypeScreen.common.css")}
                onBlur={(event) => setCssContent(event.target.value)}
            />
        </SimpleForm>
    )
}
