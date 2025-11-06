// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useRef, useState} from "react"
import {useTranslation} from "react-i18next"
import Editor from "@/components/Editor"
import {Tabs, Tab} from "@mui/material"
import {CustomTabPanel} from "@/components/CustomTabPanel"
import {TextInput, useInput} from "react-admin"

type EmailEditEditorProps = {
    sourceSubject?: string
    sourceBodyPlainText?: string
    sourceBodyHTML?: string
}

const CustomRichTextEditor: React.FC<{source: string; label?: string}> = ({source}) => {
    const {field} = useInput({source})
    const editorRef = useRef<any>(null)

    function handleHtmlChange() {
        if (editorRef.current) {
            field.onChange(editorRef.current.getContent())
        }
    }

    return <Editor editorRef={editorRef} value={field.value} onEditorChange={handleHtmlChange} />
}

export default function EmailEditEditor({
    sourceSubject,
    sourceBodyHTML,
    sourceBodyPlainText,
}: EmailEditEditorProps) {
    const {t} = useTranslation()
    const [tab, setTab] = useState<number>(0)

    const changeTab = (_event: React.SyntheticEvent, newValue: number) => {
        setTab(newValue)
    }

    return (
        <>
            {sourceSubject && <TextInput label={t("emailEditor.subject")} source={sourceSubject} />}
            <Tabs value={tab} onChange={changeTab}>
                {sourceBodyHTML && (
                    <Tab key="richtext" label={t("emailEditor.tabs.richtext")} id="richtext" />
                )}
                {sourceBodyPlainText && (
                    <Tab key="plaintext" label={t("emailEditor.tabs.plaintext")} id="plaintext" />
                )}
            </Tabs>
            {sourceBodyHTML && (
                <CustomTabPanel key="richtext" value={tab} index={0}>
                    <CustomRichTextEditor source={sourceBodyHTML} />
                </CustomTabPanel>
            )}
            {sourceBodyPlainText && (
                <CustomTabPanel key="plaintext" value={tab} index={sourceBodyHTML ? 1 : 0}>
                    <TextInput
                        label={t("emailEditor.tabs.plaintext")}
                        source={sourceBodyPlainText}
                        multiline={true}
                        minRows={6}
                        maxRows={20}
                    />
                </CustomTabPanel>
            )}
        </>
    )
}
