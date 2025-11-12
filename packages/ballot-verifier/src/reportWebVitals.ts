// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {MetricType} from "web-vitals"

const reportWebVitals = (onPerfEntry?: (metric: MetricType) => void) => {
    if (onPerfEntry && onPerfEntry instanceof Function) {
        import("web-vitals").then(({onCLS, onINP, onFCP, onLCP, onTTFB}) => {
            onCLS(onPerfEntry)
            onINP(onPerfEntry)
            onFCP(onPerfEntry)
            onLCP(onPerfEntry)
            onTTFB(onPerfEntry)
        })
    }
}

export default reportWebVitals
