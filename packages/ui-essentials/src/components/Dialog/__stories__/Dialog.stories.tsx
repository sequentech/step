// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {Meta, StoryObj} from "@storybook/react"
import Dialog, {DialogProps} from "../Dialog"
import {INITIAL_VIEWPORTS} from "@storybook/addon-viewport"
import {Button} from "@mui/material"
import {useTranslation} from "react-i18next"

interface DialogExampleProps {
    variant?: DialogProps["variant"]
    close?: boolean
}

const DialogExample: React.FC<DialogExampleProps> = ({variant, close}) => {
    const {t} = useTranslation()
    const [open, setOpen] = useState(true)

    return (
        <>
            <Button onClick={() => setOpen(true)}>{t("stories.openDialog")}</Button>
            <Dialog
                handleClose={() => setOpen(false)}
                open={open}
                title={t("ballotSelectionsScreen.statusModal.title")}
                ok={t("ballotSelectionsScreen.statusModal.ok")}
                cancel={close ? t("logout.modal.close") : undefined}
                variant={variant}
            >
                <p>{t("ballotSelectionsScreen.statusModal.content")}</p>
            </Dialog>
        </>
    )
}

const meta: Meta<typeof DialogExample> = {
    title: "components/Dialog",
    component: DialogExample,
    parameters: {
        backgrounds: {
            default: "white",
        },
        viewport: {
            viewports: INITIAL_VIEWPORTS,
            defaultViewport: "iphone6",
        },
    },
}

export default meta

type Story = StoryObj<typeof DialogExample>

export const Info: Story = {
    // More on args: https://storybook.js.org/docs/react/writing-stories/args
    args: {
        variant: "info",
    },
    parameters: {
        viewport: {
            disable: true,
        },
    },
}

export const InfoMobile: Story = {
    // More on args: https://storybook.js.org/docs/react/writing-stories/args
    args: {
        variant: "info",
    },
    parameters: {
        viewport: {
            defaultViewport: "iphone6",
        },
    },
}

export const Warning: Story = {
    // More on args: https://storybook.js.org/docs/react/writing-stories/args
    args: {
        variant: "warning",
        close: true,
    },
    parameters: {
        viewport: {
            disable: true,
        },
    },
}

export const WarningMobile: Story = {
    // More on args: https://storybook.js.org/docs/react/writing-stories/args
    args: {
        variant: "warning",
        close: true,
    },
    parameters: {
        viewport: {
            defaultViewport: "iphone6",
        },
    },
}
export const Action: Story = {
    // More on args: https://storybook.js.org/docs/react/writing-stories/args
    args: {
        variant: "action",
        close: true,
    },
    parameters: {
        viewport: {
            disable: true,
        },
    },
}

export const ActionMobile: Story = {
    // More on args: https://storybook.js.org/docs/react/writing-stories/args
    args: {
        variant: "action",
        close: true,
    },
    parameters: {
        viewport: {
            defaultViewport: "iphone6",
        },
    },
}
