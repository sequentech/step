// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {Identifier, RaRecord} from "react-admin"
import {IRole} from "@sequentech/ui-core"
import {EditUserForm} from "./EditUserForm"
import {UserProfileAttribute, UserProfileAttributeGroup} from "@/gql/graphql"

interface EditUserProps {
    id?: string
    electionEventId?: string
    electionId?: string
    close?: () => void
    rolesList: Array<IRole>
    userAttributes: UserProfileAttribute[]
    userAttributeGroups: UserProfileAttributeGroup[]
    record?: RaRecord<Identifier>
    onTaskLaunched?: (taskExecutionId: string) => void
}

export const EditUser: React.FC<EditUserProps> = ({
    id,
    close,
    electionEventId,
    electionId,
    rolesList,
    userAttributes,
    userAttributeGroups,
    record,
    onTaskLaunched,
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
                userAttributeGroups={userAttributeGroups}
                record={record}
                onTaskLaunched={onTaskLaunched}
            />
        )
    } else {
        return null
    }
}
