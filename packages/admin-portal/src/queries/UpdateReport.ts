import {gql} from "@apollo/client"

export const UPDATE_REPORT = gql`
    mutation UpdateReport($id: uuid!, $set: sequent_backend_report_set_input!) {
        update_sequent_backend_report_by_pk(pk_columns: {id: $id}, _set: $set) {
            id
        }
    }
`
