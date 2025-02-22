// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {Identifier, RaRecord} from "react-admin"
import {IRole} from "@sequentech/ui-core"
import {EditUserForm} from "./EditUserForm"
import {UserProfileAttribute} from "@/gql/graphql"

interface EditUserProps {
    id?: string
    electionEventId?: string
    electionId?: string
    close?: () => void
    rolesList: Array<IRole>
    userAttributes: UserProfileAttribute[]
    record?: RaRecord<Identifier>
}

export const EditUser: React.FC<EditUserProps> = ({
    id,
    close,
    electionEventId,
    electionId,
    rolesList,
    userAttributes,
    record,
}) => {
    const [renderUI, setRenderUI] = useState(true)

    useEffect(() => {
        if (record) {
            setRenderUI(true)
        }
    }, [record])

    if (renderUI) {
        return (
            <EditUserForm
                id={id}
                electionEventId={electionEventId}
                electionId={electionId}
                close={close}
                rolesList={rolesList}
                userAttributes={userAttributes}
                record={record}
            />
        )
    } else {
        return null
    }
}
