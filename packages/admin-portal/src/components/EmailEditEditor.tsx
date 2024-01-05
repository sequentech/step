import React, {useContext, useRef, useState} from "react"
import {useTranslation} from "react-i18next"
import Editor from "@/components/Editor"
import {Tabs, Tab} from "@mui/material"
import {CustomTabPanel} from "@/components/CustomTabPanel"
import {Identifier, RaRecord, TextInput, useInput} from "react-admin"
import {Sequent_Backend_Communication_Template} from "@/gql/graphql"

type EmailEditEditorProps = {
    record: RaRecord<Identifier> | Omit<RaRecord<Identifier>, "id">
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

export default function EmailEditEditor({record}: EmailEditEditorProps) {
    const {t} = useTranslation()

    const [tab, setTab] = useState<number>(0)

    const changeTab = (_event: React.SyntheticEvent, newValue: number) => {
        setTab(newValue)
    }

    return (
        <>
            <TextInput label={t("emailEditor.subject")} source="template.email.subject" />
            <Tabs value={tab} onChange={changeTab}>
                <Tab key="plaintext" label={t("emailEditor.tabs.plaintext")} id="plaintext" />
                <Tab key="richtext" label={t("emailEditor.tabs.richtext")} id="richtext" />
            </Tabs>
            <CustomTabPanel key="plaintext" value={tab} index={0}>
                <TextInput
                    label={t("emailEditor.tabs.plaintext")}
                    source="template.email.plaintext_body"
                    multiline={true}
                    minRows={6}
                />
            </CustomTabPanel>
            <CustomTabPanel key="richtext" value={tab} index={1}>
                <CustomRichTextEditor source="template.email.html_body" />
            </CustomTabPanel>
        </>
    )
}
