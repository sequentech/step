// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {MOCK_PATHS, type MockAbortReason, type MockRequest, type MockResponse} from "./http"
import {ViolationLog} from "./violations"

export type S3Bucket = "public" | "private"

export interface S3RequestRecord {
    method: string
    url: string
    bucket: S3Bucket
    key: string
    query: Record<string, string>
    headers: Record<string, string>
    status: number | "aborted"
}

export type S3Override = {status: number; body?: string} | {abort: MockAbortReason}

export type S3Matcher = (request: Omit<S3RequestRecord, "status">) => boolean

interface OverrideRule {
    matches: S3Matcher
    response: S3Override
    remaining: number
}

interface StoredObject {
    body: Uint8Array
    contentType: string
}

export interface S3MockOptions {
    origin: string
    basePath?: string
    violations: ViolationLog
}

/**
 * An in-memory object store addressed as `<basePath>/<bucket>/<key>`. Presigned
 * query parameters are recorded but not verified, so a test can expire a URL by
 * overriding the requests that carry its signature.
 */
export class S3Mock {
    readonly basePath: string
    readonly requests: S3RequestRecord[] = []
    private readonly origin: string
    private readonly objects = new Map<string, StoredObject>()
    private readonly overrides: OverrideRule[] = []
    private readonly violations: ViolationLog

    constructor({origin, basePath = MOCK_PATHS.s3, violations}: S3MockOptions) {
        this.origin = origin
        this.basePath = basePath
        this.violations = violations
    }

    /** The value for a portal's `PUBLIC_BUCKET_URL` setting. */
    publicBucketUrl(): string {
        return `${this.origin}${this.basePath}/public/`
    }

    url(bucket: S3Bucket, key: string, query: Record<string, string> = {}): string {
        const url = new URL(`${this.origin}${this.basePath}/${bucket}/${key}`)
        for (const [name, value] of Object.entries(query)) {
            url.searchParams.set(name, value)
        }
        return url.toString()
    }

    /** A URL shaped like an S3 presigned GET; `signature` tells attempts apart. */
    presign(key: string, signature: string, expiresSecs = 300): string {
        return this.url("private", key, {
            "X-Amz-Algorithm": "AWS4-HMAC-SHA256",
            "X-Amz-Expires": String(expiresSecs),
            "X-Amz-SignedHeaders": "host",
            "X-Amz-Signature": signature,
        })
    }

    putJson(bucket: S3Bucket, key: string, value: unknown): string {
        return this.putBytes(
            bucket,
            key,
            new TextEncoder().encode(JSON.stringify(value)),
            "application/json"
        )
    }

    putBytes(bucket: S3Bucket, key: string, body: Uint8Array, contentType: string): string {
        this.objects.set(`${bucket}/${key}`, {body, contentType})
        return this.url(bucket, key)
    }

    delete(bucket: S3Bucket, key: string): void {
        this.objects.delete(`${bucket}/${key}`)
    }

    has(bucket: S3Bucket, key: string): boolean {
        return this.objects.has(`${bucket}/${key}`)
    }

    /**
     * Answers matching requests with `response` instead of the stored object,
     * `times` times (always when omitted).
     */
    override(matches: S3Matcher, response: S3Override, times = Infinity): void {
        this.overrides.push({matches, response, remaining: times})
    }

    clearOverrides(): void {
        this.overrides.length = 0
    }

    requestsFor(key: string): S3RequestRecord[] {
        return this.requests.filter((request) => request.key === key)
    }

    handles(url: URL): boolean {
        return url.origin === this.origin && url.pathname.startsWith(`${this.basePath}/`)
    }

    handle(request: MockRequest): MockResponse {
        const path = decodeURIComponent(request.url.pathname.slice(this.basePath.length + 1))
        const separator = path.indexOf("/")
        const bucketName = separator === -1 ? path : path.slice(0, separator)
        const key = separator === -1 ? "" : path.slice(separator + 1)
        if (bucketName !== "public" && bucketName !== "private") {
            this.violations.add(`S3 request for unknown bucket: ${request.url}`)
            return {status: 404, body: "NoSuchBucket"}
        }
        const record: Omit<S3RequestRecord, "status"> = {
            method: request.method,
            url: request.url.toString(),
            bucket: bucketName,
            key,
            query: Object.fromEntries(request.url.searchParams),
            headers: request.headers,
        }
        const rule = this.overrides.find((candidate) => {
            return candidate.remaining > 0 && candidate.matches(record)
        })
        if (rule) {
            rule.remaining -= 1
            if ("abort" in rule.response) {
                this.requests.push({...record, status: "aborted"})
                return {abort: rule.response.abort}
            }
            this.requests.push({...record, status: rule.response.status})
            return {
                status: rule.response.status,
                headers: {"content-type": "application/xml"},
                body: rule.response.body ?? "",
            }
        }
        if (request.method !== "GET" && request.method !== "HEAD") {
            this.violations.add(`S3 ${request.method} is not supported: ${request.url}`)
            this.requests.push({...record, status: 405})
            return {status: 405, body: "MethodNotAllowed"}
        }
        const stored = this.objects.get(`${bucketName}/${key}`)
        if (!stored) {
            this.violations.add(`S3 GET for a missing object: ${bucketName}/${key}`)
            this.requests.push({...record, status: 404})
            return {status: 404, headers: {"content-type": "application/xml"}, body: "NoSuchKey"}
        }
        this.requests.push({...record, status: 200})
        return {
            status: 200,
            headers: {"content-type": stored.contentType, "cache-control": "private, max-age=300"},
            body: request.method === "HEAD" ? undefined : stored.body,
        }
    }
}
