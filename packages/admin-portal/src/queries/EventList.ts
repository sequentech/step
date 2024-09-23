import {gql} from "@apollo/client"

export const getEventListVariables = (input: any) => {
    return {
        filter: input?.where?._and?.reduce((acc: any, condition: any) => {
            Object.keys(condition).forEach((key) => {
                if (key !== "election_event_id") {
                    acc[key] = condition[key]?._eq
                }
            })
            console.log("acc", acc)

            return acc
        }, {}),
        ...input,
    }
}

export const getEventList = (fields: any) => {
    let election_event_id = fields?.filter?.election_event_id ?? ""
    let tenant_id = fields?.filter?.tenant_id ?? ""

    return gql`
        query GetEventList(
            $tenant_id: String! = "${tenant_id}"
            $election_event_id: String! = "${election_event_id}"
            $limit: Int
            $offset: Int
            $filter: EventListFilter
            $order_by: EventListOrderBy
        ) {
            get_event_list(
                tenant_id: $tenant_id
                election_event_id: $election_event_id
                limit: $limit
                offset: $offset 
                filter: $filter
                order_by: $order_by
            ) {
                items {
                    election
                    schedule
                    id
                    tenant_id
                    election_event_id
                    event_type
                    receivers
                    template
                    name
                }
                total {
                    aggregate {
                        count
                    }
                }
            }
        }
    `
}
