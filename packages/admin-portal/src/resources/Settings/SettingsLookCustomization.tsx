// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"

import styled from "@emotion/styled"

import {useTranslation} from "react-i18next"
import {SimpleForm, TextInput, useEditController} from "react-admin"

import {useTenantStore} from "@/providers/TenantContextProvider"
import {ITenantSettings} from "@sequentech/ui-core"

export const SettingsLookCustomization: React.FC<void> = () => {
    const [tenantId] = useTenantStore()
    const {t, i18n} = useTranslation()
    const {record, save, isLoading} = useEditController({
        resource: "sequent_backend_tenant",
        id: tenantId,
        redirect: false,
        undoable: false,
    })

    // if (save) {
    //     save({
    //         settings: {
    //             ...((record?.settings as ITenantSettings | undefined) ?? {}),
    //             // Save CSS logo URL or anything else
    //         },
    //     })
    // }

    // if (isLoading) return null

    return (
        <SimpleForm>
            <TextInput
                resettable={true}
                source={"presentation.logo_url"} //TODO: Currently Sequent logo is not shown. Fix the logo after merging 1958 to use the blank/Sequent/custom logo with the same logic than Voting Screen on that ticket
                label={t("electionEventScreen.field.logoUrl")}
            />
            <TextInput
                resettable={true}
                source={"presentation.redirect_finish_url"}
                label={t("electionEventScreen.field.redirectFinishUrl")}
            />
            <TextInput
                resettable={true}
                multiline={true}
                source={"presentation.css"}
                label={t("electionEventScreen.field.css")}
            />
        </SimpleForm>
    )
}
