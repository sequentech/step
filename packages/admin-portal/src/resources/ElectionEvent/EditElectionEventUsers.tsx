import React from "react"
import { ListUsers } from "../User/ListUsers"
import { Sequent_Backend_Election_Event } from "../../gql/graphql"
import { useRecordContext } from "react-admin"

export const EditElectionEventUsers: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    return <ListUsers electionEventId={record?.id}/>
}
