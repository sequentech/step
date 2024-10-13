import {gql} from "@apollo/client"

export const GENERATE_REPORT = gql`
    mutation GenerateReport($reportId: String!, $reportMode: String!, $tenantId: String!) {
        generate_report(report_id: $reportId, report_mode: $reportMode, tenant_id: $tenantId) {
            document_id
        }
    }
`
