import React from "react"
import {useTranslation} from "react-i18next"
import {FormStyles} from "@/components/styles/FormStyles"

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
    const {t} = useTranslation()

    const handleChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
        const {name, value} = e.target
        setRecord({...(record as any), [name]: value})
    }
    if (!record) {
        return <></>
    }
    return (
        <FormStyles.TextField
            label={t("emailEditor.subject")}
            value={record?.subject}
            onChange={handleChange}
        />
    )
}
