// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {PageHeaderStyles} from "@/components/styles/PageHeaderStyles"
import React from "react"
import {IRole} from "@sequentech/ui-core"
import {Sequent_Backend_Area, UserProfileAttribute} from "@/gql/graphql"
import {EditUserForm} from "./EditUserForm"

interface CreateUserProps {
    electionEventId?: string
    close?: () => void
    userAttributes: UserProfileAttribute[]
    rolesList: Array<IRole>
    areas?: Sequent_Backend_Area[]
}

export const CreateUser: React.FC<CreateUserProps> = ({
    electionEventId,
    close,
    userAttributes,
    rolesList,
    areas,
}) => {
    return (
        <PageHeaderStyles.Wrapper>
            <EditUserForm
                rolesList={rolesList}
                userAttributes={userAttributes}
                close={close}
                createMode
                electionEventId={electionEventId}
                areas={areas}
            />
        </PageHeaderStyles.Wrapper>
    )
}
