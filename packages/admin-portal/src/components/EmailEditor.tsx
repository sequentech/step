// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useRef, useState} from "react"
import {useTranslation} from "react-i18next"
import Editor from "@/components/Editor"
import {Tabs, Tab} from "@mui/material"
import {FormStyles} from "@/components/styles/FormStyles"
import {CustomTabPanel} from "@/components/CustomTabPanel"
import {IEmail} from "@/types/templates"

interface EmailEditorProps {
    record: IEmail
    setRecord: (newRecord: IEmail) => void
}

export default function EmailEditor({record, setRecord}: EmailEditorProps) {
    const {t} = useTranslation()

    const [tab, setTab] = useState<number>(0)

    const editorRef = useRef<any>(null)

    function handleChange(e: React.ChangeEvent<HTMLInputElement>) {
        const {name, value} = e.target
        setRecord({...(record as any), [name]: value})
    }

    function handleHtmlChange() {
        if (editorRef.current) {
            setRecord({...(record as any), html_body: editorRef.current.getContent()})
        }
    }

    const changeTab = (_event: React.SyntheticEvent, newValue: number) => {
        setTab(newValue)
    }

    return (
        <>
            <FormStyles.TextField
                label={t("emailEditor.subject")}
                name="subject"
                value={record?.subject}
                onChange={handleChange}
            />
            <Tabs value={tab} onChange={changeTab}>
                <Tab key="plaintext" label={t("emailEditor.tabs.plaintext")} id="plaintext" />
                <Tab key="richtext" label={t("emailEditor.tabs.richtext")} id="richtext" />
            </Tabs>
            <CustomTabPanel key="plaintext" value={tab} index={0}>
                <FormStyles.TextField
                    name="plaintext_body"
                    label={t("emailEditor.tabs.plaintext")}
                    value={record?.plaintext_body}
                    onChange={handleChange}
                    multiline={true}
                    minRows={6}
                    maxRows={20}
                />
            </CustomTabPanel>
            <CustomTabPanel key="richtext" value={tab} index={1}>
                <Editor
                    editorRef={editorRef}
                    initialValue={record.html_body || ""}
                    onEditorChange={handleHtmlChange}
                ></Editor>
            </CustomTabPanel>
        </>
    )
}
