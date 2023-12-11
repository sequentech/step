import React, {useRef, useState} from "react"
import {useTranslation} from "react-i18next"
//import BundledEditor from './BundledEditor'
import {Tabs, Tab} from "@mui/material"
import {FormStyles} from "@/components/styles/FormStyles"
import {CustomTabPanel} from "@/components/CustomTabPanel"
import globalSettings from "@/global-settings"

interface Email {
    subject: string
    plaintext_body: string
    html_body: string
}

type EmailEditorProps = {
    record: Email | undefined
    setRecord: (newRecord: Email) => void
}

export const EmailEditor: React.FC<EmailEditorProps> = ({record, setRecord}) => {
    const editorRef = useRef<any>(null)
    const {t} = useTranslation()
    const [tab, setTab] = useState<number>(0)

    const handleChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
        const {name, value} = e.target
        setRecord({...(record as any), [name]: value})
    }
    const changeTab = (event: React.SyntheticEvent, newValue: number) => {
        setTab(newValue)
    }

    if (!record) {
        return <></>
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
                />
            </CustomTabPanel>
            <CustomTabPanel key="richtext" value={tab} index={1}>
                {/*<BundledEditor
                    onInit={(evt: any, editor: any) => editorRef.current = editor}
                    initialValue={globalSettings.DEFAULT_EMAIL_HTML_BODY}
                    init={{
                        height: 500,
                        menubar: true,
                        plugins: [
                            'advlist autolink lists link image charmap preview anchor',
                            'searchreplace visualblocks code fullscreen',
                            'insertdatetime media table paste code help wordcount'
                        ],
                        toolbar: 'undo redo | formatselect | ' +
                        'bold italic backcolor | alignleft aligncenter ' +
                        'alignright alignjustify | bullist numlist outdent indent | ' +
                        'removeformat | help',
                        content_style: 'body { font-family:Helvetica,Arial,sans-serif; font-size:14px }'
                    }}
                />*/}
            </CustomTabPanel>
        </>
    )
}
