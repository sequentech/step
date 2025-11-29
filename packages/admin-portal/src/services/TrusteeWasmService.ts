// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {ApolloClient} from "@apollo/client"
import {RECORD_KEY_COMMITMENT} from "../queries/RecordKeyCommitment"
import {VERIFY_KEY_COMMITMENT} from "../queries/VerifyKeyCommitment"
import {GET_TRUSTEE_ARTIFACT_UPLOAD_URL} from "../queries/GetTrusteeArtifactUploadUrl"
import {GET_TRUSTEE_ARTIFACT_DOWNLOAD_URL} from "../queries/GetTrusteeArtifactDownloadUrl"

// NOTE: the actual wasm module will be provided by the bundler when importing
// the braid-wasm package. Here we only type its minimal API surface
// to keep this service decoupled from the bundler details.

export interface IBraidWasmModule {
    set_hooks: () => void
    generate_trustee_keypair_js: (
        electionId: string,
        trusteeId: string,
        iterations: number,
    ) => any
    export_private_key_file_js: (
        keyId: number,
        electionId: string,
        trusteeId: string,
        publicKeyB64: string,
        passphrase: string,
        iterations: number,
    ) => any
    import_private_key_file_js: (file: any, passphrase: string) => any
    recompute_key_commitment_js: (
        keyId: number,
        electionId: string,
        trusteeId: string,
        iterations: number,
    ) => any
    build_signed_board_message_js: (
        keyId: number,
        board: string,
        payload: Uint8Array,
        artifact: any,
        publicKeyB64: string,
    ) => any
    // New trustee engine exports
    init_trustee_js: (
        boardName: string,
        trusteeName: string,
        signingKeyId: number,
    ) => number
    trustee_step_js: (
        trusteeId: number,
        messagesBorsh: Uint8Array,
    ) => TrusteeStepResult
}

export interface KeyCommitment {
    salt_b64: string
    iterations: number
    hash_b64: string
}

export interface GeneratedKeypair {
    election_id: string
    trustee_id: string
    public_key_b64: string
    commitment: KeyCommitment
    key_id: number
}

export interface ImportedKey {
    election_id: string
    trustee_id: string
    public_key_b64: string
    key_id: number
}

export interface TrusteeArtifactUploadResult {
    url: string
    bucket: string
    key: string
}

export interface ArtifactEnvelope {
    bucket: string
    key: string
    sha256_hex: string
    size: number
    content_type: string
    kind: string
}

export interface BoardSignedMessage {
    board: string
    payload_hex: string
    artifact?: ArtifactEnvelope
    signature_b64: string
    public_key_b64: string
}

export interface TrusteeStepResult {
    outgoing_messages_b64: string[]
    last_message_id: number
    added_messages: number
}

export class TrusteeWasmService {
    private wasm: IBraidWasmModule
    private apollo: ApolloClient<unknown>

    constructor(wasm: IBraidWasmModule, apollo: ApolloClient<unknown>) {
        this.wasm = wasm
        this.apollo = apollo
        this.wasm.set_hooks()
    }

    async generateKeypair(
        electionEventId: string,
        trusteeName: string,
        commitmentIterations: number,
    ): Promise<GeneratedKeypair> {
        const result = this.wasm.generate_trustee_keypair_js(
            electionEventId,
            trusteeName,
            commitmentIterations,
        ) as GeneratedKeypair

        // Persist PBKDF2 commitment server-side so we can later verify key
        // files without ever sending the private key.
        await this.apollo.mutate({
            mutation: RECORD_KEY_COMMITMENT,
            variables: {
                electionEventId,
                trusteeName,
                saltB64: result.commitment.salt_b64,
                iterations: result.commitment.iterations,
                hashB64: result.commitment.hash_b64,
            },
        })

        return result
    }

    async exportKeyFile(
        keyId: number,
        electionEventId: string,
        trusteeName: string,
        publicKeyB64: string,
        passphrase: string,
        fileIterations: number,
    ): Promise<any> {
        return this.wasm.export_private_key_file_js(
            keyId,
            electionEventId,
            trusteeName,
            publicKeyB64,
            passphrase,
            fileIterations,
        )
    }

    async importKeyFile(
        fileObject: any,
        passphrase: string,
        electionEventId: string,
        trusteeName: string,
        commitmentIterations: number,
    ): Promise<ImportedKey & {isValid: boolean}> {
        const imported = this.wasm.import_private_key_file_js(
            fileObject,
            passphrase,
        ) as ImportedKey

        // Recompute commitment client-side using the same parameters as during
        // generation and ask the backend to verify it against the stored value.
        const commitment = this.wasm.recompute_key_commitment_js(
            imported.key_id,
            electionEventId,
            trusteeName,
            commitmentIterations,
        ) as KeyCommitment

        const verifyResult = await this.apollo.mutate({
            mutation: VERIFY_KEY_COMMITMENT,
            variables: {
                electionEventId,
                trusteeName,
                saltB64: commitment.salt_b64,
                iterations: commitment.iterations,
                hashB64: commitment.hash_b64,
            },
        })

        const isValid = Boolean(
            verifyResult.data?.verify_key_commitment?.is_valid ?? false,
        )

        return {...imported, isValid}
    }

    async getArtifactUploadUrl(
        electionEventId: string,
        artifactKind: string,
        fileName: string,
        mediaType: string,
        size: number,
    ): Promise<TrusteeArtifactUploadResult> {
        const result = await this.apollo.mutate({
            mutation: GET_TRUSTEE_ARTIFACT_UPLOAD_URL,
            variables: {
                electionEventId,
                artifactKind,
                fileName,
                mediaType,
                size,
            },
        })

        return result.data?.get_trustee_artifact_upload_url as TrusteeArtifactUploadResult
    }

    async getArtifactDownloadUrl(bucket: string, key: string): Promise<string> {
        const result = await this.apollo.mutate({
            mutation: GET_TRUSTEE_ARTIFACT_DOWNLOAD_URL,
            variables: {bucket, key},
        })

        return result.data?.get_trustee_artifact_download_url?.url as string
    }

    buildSignedBoardMessage(
        keyId: number,
        board: string,
        payload: Uint8Array,
        artifact: ArtifactEnvelope | null,
        publicKeyB64: string,
    ): BoardSignedMessage {
        const raw = this.wasm.build_signed_board_message_js(
            keyId,
            board,
            payload,
            artifact,
            publicKeyB64,
        ) as BoardSignedMessage

        return raw
    }

    // New helpers for running the full trustee engine in the browser.

    /**
     * Initialise a wasm-backed Trustee engine for this election/board.
     * The signingKeyId is the in-memory handle returned by importKeyFile or
     * generateKeypair.
     */
    initTrustee(boardName: string, trusteeName: string, signingKeyId: number): number {
        return this.wasm.init_trustee_js(boardName, trusteeName, signingKeyId)
    }

    /**
     * Run a single trustee step over a Borsh-encoded batch of GrpcB3Message
     * objects provided by the backend.
     */
    runTrusteeStep(trusteeId: number, messagesBorsh: Uint8Array): TrusteeStepResult {
        return this.wasm.trustee_step_js(trusteeId, messagesBorsh)
    }
}
