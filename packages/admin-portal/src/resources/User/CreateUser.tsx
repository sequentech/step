// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {PageHeaderStyles} from "@/components/styles/PageHeaderStyles"
import React from "react"
import {IRole} from "@sequentech/ui-core"
import {UserProfileAttribute, UserProfileAttributeGroup} from "@/gql/graphql"
import {EditUserForm} from "./EditUserForm"

interface CreateUserProps {
    electionEventId?: string
    close?: () => void
    userAttributes: UserProfileAttribute[]
    userAttributeGroups: UserProfileAttributeGroup[]
    rolesList: Array<IRole>
}

export const CreateUser: React.FC<CreateUserProps> = ({
    electionEventId,
    close,
    userAttributes,
    userAttributeGroups,
    rolesList,
}) => {
    return (
        <PageHeaderStyles.Wrapper>
            <EditUserForm
                rolesList={rolesList}
                userAttributes={userAttributes}
                userAttributeGroups={userAttributeGroups}
                close={close}
                createMode
                electionEventId={electionEventId}
            />
        </PageHeaderStyles.Wrapper>
    )
}
