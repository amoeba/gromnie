/* tslint:disable */
/* eslint-disable */

export class GromnieWispClient {
    free(): void;
    [Symbol.dispose](): void;
    connect(): Promise<void>;
    constructor(ws_url: string);
    open_tcp_stream(host: string, port: number): Promise<number>;
    open_udp_stream(host: string, port: number): Promise<number>;
    set_allow_v1_downgrade(allow: boolean): void;
}

export class WasmClient {
    free(): void;
    [Symbol.dispose](): void;
    connect(ws_url: string, server_host: string, account_name: string, password: string): Promise<void>;
    constructor();
    select_character(character_id: number, account: string): void;
    send_chat(message: string): void;
    set_on_event(callback: Function): void;
    set_on_net_log(callback: Function): void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_wasmclient_free: (a: number, b: number) => void;
    readonly wasmclient_connect: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number) => any;
    readonly wasmclient_new: () => number;
    readonly wasmclient_select_character: (a: number, b: number, c: number, d: number) => [number, number];
    readonly wasmclient_send_chat: (a: number, b: number, c: number) => [number, number];
    readonly wasmclient_set_on_event: (a: number, b: any) => void;
    readonly wasmclient_set_on_net_log: (a: number, b: any) => void;
    readonly __wbg_gromniewispclient_free: (a: number, b: number) => void;
    readonly gromniewispclient_connect: (a: number) => any;
    readonly gromniewispclient_new: (a: number, b: number) => number;
    readonly gromniewispclient_open_tcp_stream: (a: number, b: number, c: number, d: number) => any;
    readonly gromniewispclient_open_udp_stream: (a: number, b: number, c: number, d: number) => any;
    readonly gromniewispclient_set_allow_v1_downgrade: (a: number, b: number) => void;
    readonly wasm_bindgen__convert__closures_____invoke__he4109a779b4a0bd0: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen__convert__closures_____invoke__h1cce0fa91f66ae20: (a: number, b: number, c: any, d: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h62e4fa3f98f52b2c: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h62e4fa3f98f52b2c_2: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h7ff13b1c71c1504b: (a: number, b: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_destroy_closure: (a: number, b: number) => void;
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
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
