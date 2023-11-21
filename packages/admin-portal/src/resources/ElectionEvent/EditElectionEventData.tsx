import React from "react"
import {EditBase} from "react-admin"
import {EditElectionEventDataForm} from "./EditElectionEventDataForm"
import { isConstValueNode } from 'graphql'

export const EditElectionEventData: React.FC = () => {
    const transform = (data: any) => {
        // save presentation object
        // language_conf
        console.log("data before :: ", data)
        const enabled_language_codes = []
        for (const key in data.enabled_languages) {
            if (typeof data.enabled_languages[key] === 'boolean' && data.enabled_languages[key]) {
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
            <EditElectionEventDataForm />
        </EditBase>
    )
}
