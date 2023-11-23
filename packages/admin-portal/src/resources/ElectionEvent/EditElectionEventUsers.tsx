import React, {useContext} from "react"
import {ListUsers} from "../User/ListUsers"
import {Sequent_Backend_Election_Event} from "../../gql/graphql"
import {useRecordContext} from "react-admin"
import {AuthContext} from "../../providers/AuthContextProvider"
import { IPermissions } from "sequent-core"

export const EditElectionEventUsers: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const authContext = useContext(AuthContext)
    const showUsers = authContext.isAuthorized(false, authContext.tenantId, IPermissions.VOTER_READ)

    if (!showUsers) {
        return null
    }

    return <ListUsers electionEventId={record?.id} />
}
