// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

// What the dev fake provider routes received (plan §2.10). The action fails
// where the routes aren't mounted, and the Sent messages tab isn't shown.
// No email HTML here: it is polled every 5 s and the views need only the
// preview; `has_html` says whether the email has HTML (R6-03).
export const FAKE_PROVIDER_MESSAGES = gql`
    query MessagingFakeProviderMessages {
        messaging_fake_provider_messages {
            messages {
                id
                received_at
                channel
                provider
                sender
                recipient
                report {
                    state
                    status
                    reason
                }
                content {
                    text
                    subject
                    has_html
                    truncated
                    template_name
                    template_language
                    parameters {
                        name
                        value
                    }
                    code
                }
            }
        }
    }
`

// One entry in full, with its email HTML: read only for the email open in
// the reading pane (R6-03).
export const FAKE_PROVIDER_MESSAGE = gql`
    query MessagingFakeProviderMessage($id: String!) {
        messaging_fake_provider_message(id: $id) {
            id
            received_at
            channel
            provider
            sender
            recipient
            report {
                state
                status
                reason
            }
            content {
                text
                subject
                html
                truncated
                template_name
                template_language
                parameters {
                    name
                    value
                }
                code
            }
        }
    }
`

// Stands in for the voter's tap on Get Started in Messenger: the fake posts
// Meta's first-interaction event for the pasted m.me link to the webhook
// component, and answers with the component's status.
export const FAKE_PROVIDER_GET_STARTED = gql`
    mutation MessagingFakeProviderGetStarted($link: String!) {
        messaging_fake_provider_get_started(link: $link) {
            status
            reason
        }
    }
`
