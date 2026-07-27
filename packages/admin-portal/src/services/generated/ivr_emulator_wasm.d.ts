/* tslint:disable */
/* eslint-disable */
export interface EmulatorConfig {
    /**
     * E164 number (+132...) of the caller
     */
    caller_number: string;
    contact_id: string;
    tenant_id: string;
    election_event_id: string;
    /**
     * Jsonified election event.
     */
    election_event: string;
    /**
     * Jsonified `ballot_eml`s
     */
    ballot_styles: string[];
    /**
     * `ElectionId`s
     */
    open_elections: string[];
    /**
     * E164 numbers (+132..)
     */
    blacklisted_numbers: string[];
}

export interface PromptInfo {
    prompt_text: string;
    language: string;
    voice_id: string;
}

export type Action = { type: "Prompt"; prompt: PromptInfo } | { type: "ExpectInput"; prompt: PromptInfo; valid_inputs: string; max_digits: number; timeout: number } | { type: "Disconnect"; prompt: PromptInfo } | { type: "Noop" };


export class IvrEmulatorDriver {
    free(): void;
    [Symbol.dispose](): void;
    attributes(): any;
    execute(until_io: boolean): Promise<Action>;
    constructor(config: EmulatorConfig);
    send_input(input: string): void;
    send_timeout(): void;
}

export function init(): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly init: () => void;
    readonly __wbg_ivremulatordriver_free: (a: number, b: number) => void;
    readonly ivremulatordriver_attributes: (a: number) => [number, number, number];
    readonly ivremulatordriver_execute: (a: number, b: number) => any;
    readonly ivremulatordriver_new: (a: any) => [number, number, number];
    readonly ivremulatordriver_send_input: (a: number, b: number, c: number) => void;
    readonly ivremulatordriver_send_timeout: (a: number) => void;
    readonly ring_core_0_17_14__bn_mul_mont: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly wasm_bindgen__closure__destroy__hdb4b145fc5c0ff94: (a: number, b: number) => void;
    readonly wasm_bindgen__convert__closures_____invoke__ha2bc130ecf8461d3: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen__convert__closures_____invoke__hfcaf3c30771b308a: (a: number, b: number, c: any, d: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h563360c8d8d07198: (a: number, b: number) => number;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __externref_drop_slice: (a: number, b: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
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
export default function __wbg_init (module_or_path: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
