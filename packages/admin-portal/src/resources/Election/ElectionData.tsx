import React from "react"
import {EditBase} from "react-admin"
import {ElectionDataForm} from "./ElectionDataForm"

export const EditElectionData: React.FC = () => {
    const transform = (data: any) => {
        console.log("TRANSFORM ELECTION :: ", data)

        // save presentation object
        // language_conf
        console.log("data before :: ", data)
        const enabled_language_codes = []
        for (const key in data.enabled_languages) {
            if (typeof data.enabled_languages[key] === "boolean" && data.enabled_languages[key]) {
                enabled_language_codes.push(key)
            }
        }
        const language_conf = {
            enabled_language_codes: enabled_language_codes,
        }
        // i18n
        // is alll object, no change needed
        delete data.enabled_languages

        return {
            ...data,
            presentation: {
                ...data.presentation,
                language_conf: {...language_conf},
            },
        }
    }

    return (
        <EditBase redirect={"."} transform={transform}>
            <ElectionDataForm />
        </EditBase>
    )
}
