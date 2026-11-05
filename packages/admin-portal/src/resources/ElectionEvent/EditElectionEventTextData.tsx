// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Sequent_Backend_Election_Event_Extended} from "./EditElectionEventDataForm"
import {EditBase, Identifier, RaRecord} from "react-admin"
import EditElectionEventTextDataTable from "./EditElectionEventTextDataTable"

const EditElectionEventTextData = () => {
    const transform = (data: Sequent_Backend_Election_Event_Extended): RaRecord<Identifier> => {
        console.log("TRANSFORM :: ", data)
        // save presentation object
        // language_conf
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

        // The name and alias live only in the presentation (migration 1772358027729).
        const fromPresentationDescription =
            data?.presentation?.i18n?.en?.description ||
            data?.presentation?.i18n[Object.keys(data.presentation.i18n)[0]].description ||
            ""
        data.description = fromPresentationDescription

        return {
            ...data,
            presentation: {
                ...data.presentation,
                language_conf: {
                    ...language_conf,
                    default_language_code: data?.presentation?.language_conf?.default_language_code,
                },
            },
        }
    }
    return (
        <EditBase redirect={"."} transform={transform}>
            <EditElectionEventTextDataTable />
        </EditBase>
    )
}

export default EditElectionEventTextData
