// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import init, * as wasm from "./capture-wasm/index.js"
import wasmUrl from "./capture-wasm/index_bg.wasm?url"
import modelUrl from "./capture-wasm/face_detection_yunet.onnx?url"
import {
    DocumentStatus,
    FaceStatus,
    type Analyzers,
    type DocumentAnalyzer,
    type FaceAnalyzer,
} from "./types"

let initialized: Promise<unknown> | undefined

function parseStatus<Status extends string>(values: Record<string, Status>, value: string): Status {
    const status = Object.values(values).find((known) => known === value)
    if (status === undefined) {
        throw new Error(`The id-capture package returned an unknown status ${value}`)
    }
    return status
}

function documentAnalyzer(analyzer: wasm.DocumentAnalyzer): DocumentAnalyzer {
    return {
        analyze: (...args) => {
            const frame = analyzer.analyze(...args)
            return {...frame, status: parseStatus(DocumentStatus, frame.status)}
        },
        checkStill: (...args) => {
            const still = analyzer.checkStill(...args)
            return {...still, status: parseStatus(DocumentStatus, still.status)}
        },
        reset: () => analyzer.reset(),
        free: () => analyzer.free(),
    }
}

function faceAnalyzer(analyzer: wasm.FaceAnalyzer): FaceAnalyzer {
    return {
        analyze: (...args) => {
            const frame = analyzer.analyze(...args)
            return {...frame, status: parseStatus(FaceStatus, frame.status)}
        },
        reset: () => analyzer.reset(),
        free: () => analyzer.free(),
    }
}

export async function loadAnalyzers(): Promise<Analyzers> {
    initialized ??= init({module_or_path: wasmUrl})
    try {
        await initialized
    } catch (error) {
        initialized = undefined
        throw error
    }
    const response = await fetch(modelUrl)
    if (!response.ok) {
        throw new Error(`The face detection model could not be loaded: ${response.status}`)
    }
    const model = new Uint8Array(await response.arrayBuffer())
    return {
        document: documentAnalyzer(new wasm.DocumentAnalyzer()),
        face: faceAnalyzer(new wasm.FaceAnalyzer(model)),
    }
}
