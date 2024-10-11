import {gql} from "@apollo/client"

export const CREATE_REPORT = gql`
    mutation InsertReport($object: sequent_backend_report_insert_input!) {
        insert_sequent_backend_report(objects: [$object]) {
            affected_rows
        }
    }
`
