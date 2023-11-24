import React, {useContext} from "react"
import {ListUsers} from "../User/ListUsers"
import {Sequent_Backend_Election_Event} from "../../gql/graphql"
import {useRecordContext} from "react-admin"
import {AuthContext} from "../../providers/AuthContextProvider"

export const EditElectionEventUsers: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const authContext = useContext(AuthContext)
    const showUsers = authContext.hasPermissions(false, authContext.tenantId, "read-event-users")

    if (!showUsers) {
        return null
    }

    return <ListUsers electionEventId={record?.id} />
}
