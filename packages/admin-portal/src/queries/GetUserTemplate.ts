// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const GET_USER_TEMPLATE = gql`
    mutation GetUserTemplate($template_type: String!) {
        get_user_template(template_type: $template_type) {
            template_hbs
            extra_config
        }
    }
`
