// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Preview} from "@storybook/react-vite"

const preview: Preview = {
    parameters: {layout: "fullscreen", a11y: {test: "error"}},
    initialGlobals: {locale: "en"},
    globalTypes: {
        locale: {
            toolbar: {
                title: "Locale",
                icon: "globe",
                items: [
                    {value: "en", title: "English"},
                    {value: "es", title: "Spanish"},
                ],
            },
        },
    },
    decorators: [
        (Story, context) => (
            <Story
                args={{...context.args, locale: context.args.locale ?? context.globals.locale}}
            />
        ),
    ],
    beforeEach: () => {
        localStorage.removeItem("resendOtpEndTime")
    },
}

export default preview
