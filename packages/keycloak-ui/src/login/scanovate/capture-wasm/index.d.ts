/* tslint:disable */
/* eslint-disable */

export type Point = [number, number];
export type DocumentStatus =
| "NO_DOCUMENT" | "TOO_FAR" | "TOO_CLOSE" | "NOT_ALIGNED" | "TILTED"
| "TOO_DARK" | "TOO_BRIGHT" | "GLARE" | "BLURRY" | "HOLD_STILL" | "READY";
export interface DocumentFrame {
    status: DocumentStatus;
    corners: Point[] | null;
    fill: number;
    sharpness: number;
    glare: number;
    brightness: number;
    stability: number;
}
export interface StillCheck {
    status: DocumentStatus;
    corners: Point[] | null;
    cardWidth: number;
    blur: number;
}
export type FaceStatus =
| "NO_FACE" | "MULTIPLE_FACES" | "TOO_FAR" | "TOO_CLOSE" | "OFF_CENTER"
| "TURN_TO_CAMERA" | "TOO_DARK" | "TOO_BRIGHT" | "BLURRY" | "HOLD_STILL" | "READY";
export interface FaceFrame {
    status: FaceStatus;
    box: {x: number; y: number; width: number; height: number} | null;
    landmarks: Point[];
    yaw: number;
    roll: number;
    brightness: number;
    sharpness: number;
    stability: number;
}



/**
 * Document analyzer exposed to JavaScript as `DocumentAnalyzer`.
 */
export class DocumentAnalyzer {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Analyses a downscaled RGBA frame against the guide rectangle.
     *
     * # Errors
     *
     * Throws when the buffer does not hold `width * height` RGBA pixels or the guide is invalid.
     */
    analyze(rgba: Uint8Array | Uint8ClampedArray, width: number, height: number, guideX: number, guideY: number, guideWidth: number, guideHeight: number): DocumentFrame;
    /**
     * Checks the full resolution RGBA still that is uploaded, against the guide rectangle in its
     * pixel coordinates. The stillness history is left alone.
     *
     * # Errors
     *
     * Throws when the buffer does not hold `width * height` RGBA pixels or the guide is invalid.
     */
    checkStill(rgba: Uint8Array | Uint8ClampedArray, width: number, height: number, guideX: number, guideY: number, guideWidth: number, guideHeight: number): StillCheck;
    /**
     * Analyzer with no history.
     */
    constructor();
    /**
     * Forgets the stillness history.
     */
    reset(): void;
}

/**
 * Face analyzer exposed to JavaScript as `FaceAnalyzer`.
 */
export class FaceAnalyzer {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Analyses an RGBA frame against the oval the face must fill.
     *
     * # Errors
     *
     * Throws when the buffer does not hold `width * height` RGBA pixels, the oval is invalid or
     * inference fails.
     */
    analyze(rgba: Uint8Array | Uint8ClampedArray, width: number, height: number, ovalCenterX: number, ovalCenterY: number, ovalRadiusX: number, ovalRadiusY: number): FaceFrame;
    /**
     * Loads the `YuNet` ONNX model.
     *
     * # Errors
     *
     * Throws when the bytes are not a `YuNet` model the runtime can execute.
     */
    constructor(model: Uint8Array);
    /**
     * Forgets the stillness history.
     */
    reset(): void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_documentanalyzer_free: (a: number, b: number) => void;
    readonly __wbg_faceanalyzer_free: (a: number, b: number) => void;
    readonly documentanalyzer_analyze: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number) => void;
    readonly documentanalyzer_checkStill: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number) => void;
    readonly documentanalyzer_new: () => number;
    readonly documentanalyzer_reset: (a: number) => void;
    readonly faceanalyzer_analyze: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number) => void;
    readonly faceanalyzer_new: (a: number, b: number, c: number) => void;
    readonly faceanalyzer_reset: (a: number) => void;
    readonly __wbindgen_export: (a: number, b: number) => number;
    readonly __wbindgen_export2: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_export3: (a: number) => void;
    readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
