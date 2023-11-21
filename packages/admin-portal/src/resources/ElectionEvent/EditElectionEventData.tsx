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
            console.log('key :>> ', key);
            console.log("key type :>> ", typeof data.enabled_languages[key])
            if (typeof data.enabled_languages[key] === 'boolean' && data.enabled_languages[key]) {
                enabled_language_codes.push(key)
            }
        }
        const language_conf = {
            enabled_language_codes: enabled_language_codes,
        }
        // i18n
        const i18n = {}

        console.log("data to send :: ", {
            ...data,
            presentation: {
                language_conf: {...language_conf},
                i18n: {...i18n},
            },
        })

        return {
            ...data,
            presentation: {
                language_conf: {...language_conf},
                i18n: {...i18n},
            },
        }
    }

    return (
        <EditBase redirect={"."} transform={transform}>
            <EditElectionEventDataForm />
        </EditBase>
    )
}
