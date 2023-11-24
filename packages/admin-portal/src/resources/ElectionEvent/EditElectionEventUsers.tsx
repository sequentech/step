import React, {useContext} from "react"
import {ListUsers} from "../User/ListUsers"
import {Sequent_Backend_Election_Event} from "../../gql/graphql"
import {useRecordContext} from "react-admin"
import {AuthContext} from "../../providers/AuthContextProvider"
//import { IPermissions } from "sequent-core"
import {useTenantStore} from "../../providers/TenantContextProvider"

export const EditElectionEventUsers: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const authContext = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    const showUsers = authContext.isAuthorized(true, tenantId, "voter-read")

    if (!showUsers) {
        return null
    }

    return <ListUsers electionEventId={record?.id} />
}
