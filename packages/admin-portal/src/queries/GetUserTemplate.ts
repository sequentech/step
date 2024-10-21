// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const GET_USER_TEMPLATE = gql`
    mutation GetUserTemplate($template_name: String!) {
        get_user_template(template_name: $template_name) {
            template_hbs
        }
    }
`
