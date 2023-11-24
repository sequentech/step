// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {Edit, Identifier, useListContext} from "react-admin"
import {PageHeaderStyles} from "../../components/styles/PageHeaderStyles"
import ElectionHeader from "../../components/ElectionHeader"
import {useTranslation} from "react-i18next"
import {IRole} from "sequent-core"

interface EditRoleProps {
    id?: Identifier | undefined
    close?: () => void
}

export const EditRole: React.FC<EditRoleProps> = ({id, close}) => {
    const {data, isLoading} = useListContext()
    const {t} = useTranslation()
    if (isLoading || !data) {
        return null
    }
    let role: IRole | undefined = data?.find((element) => element.id === id)

    return (
        <PageHeaderStyles.Wrapper>
            <ElectionHeader
                title={t("usersAndRolesScreen.roles.edit.title")}
                subtitle="usersAndRolesScreen.roles.edit.subtitle"
            />
            {role?.id}
        </PageHeaderStyles.Wrapper>
    )
}
