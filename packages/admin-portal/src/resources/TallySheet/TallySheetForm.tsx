// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {TextInput} from "react-admin"
import {PageHeaderStyles} from "../../components/styles/PageHeaderStyles"
import {useTranslation} from "react-i18next"

export const TallySheetForm: React.FC = () => {
    const {t} = useTranslation()

    return (
        <>
            <PageHeaderStyles.Title>{t("areas.common.title")}</PageHeaderStyles.Title>
            <PageHeaderStyles.SubTitle>{t("areas.common.subTitle")}</PageHeaderStyles.SubTitle>

            <TextInput source="name" />
            <TextInput source="description" />
        </>
    )
}
