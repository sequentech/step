// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The explorers an election's Vitrine policy names: static sites that read the
// public record themselves, run by its monitors and hubs (independent of the
// election authority) and by the authority (the operator).
//
// Stand-in for `explorersOf` and `explorerLink` of `@vitrine/verify`, which the
// vendored package lacks until it is updated: once it has them, import them
// from there in place of this module and delete it. The signatures are the
// package's.

import type {Policy} from "@vitrine/verify"

export type TExplorerKind = "monitor" | "hub" | "operator"

export interface IExplorer {
    /** The explorer's address: an https origin. */
    url: string
    kind: TExplorerKind
    /** The monitor or the hub that runs it, or the operator. */
    name: string
    /** Who is responsible for it. */
    party: string
    /** Whether someone other than the election authority runs it. */
    independent: boolean
}

export interface IExplorerLinkTarget {
    policyId: string
    /** The name of the log in the policy, such as `ballots`. */
    log: string
    /** A Tracking ID, to find its ballot. */
    find?: string
    /** An entry's index in the log. */
    entry?: number
}

/** The policy, as the next version of the package has it: its entries may
 * name an explorer. */
interface IPolicyWithExplorers {
    monitors?: Map<string, {name: string; party: string; explorer?: string}>
    hubs?: Map<string, {name: string; operator: string; explorer?: string}>
    operator?: string
    explorers?: string[]
}

/** The policy's explorers: those of its monitors and hubs first, independent
 * of the operator, then the operator's. Empty when the policy names none. */
export const explorersOf = (policy: Policy | undefined): IExplorer[] => {
    const p = policy as IPolicyWithExplorers | undefined
    const independent: IExplorer[] = []
    for (const monitor of Array.from(p?.monitors?.values() ?? [])) {
        if (monitor.explorer) {
            independent.push({
                url: monitor.explorer,
                kind: "monitor",
                name: monitor.name,
                party: monitor.party,
                independent: true,
            })
        }
    }
    for (const hub of Array.from(p?.hubs?.values() ?? [])) {
        if (hub.explorer) {
            independent.push({
                url: hub.explorer,
                kind: "hub",
                name: hub.name,
                party: hub.operator,
                independent: true,
            })
        }
    }
    const operator = (p?.explorers ?? []).map(
        (url): IExplorer => ({
            url,
            kind: "operator",
            name: p?.operator ?? "",
            party: p?.operator ?? "",
            independent: false,
        })
    )
    return [...independent, ...operator]
}

/** Where an explorer opens a ballot or an entry:
 * `<url>#policy=<policy id>&log=<log name>&find=<Tracking ID>` (or `&entry=<index>`). */
export const explorerLink = (url: string, target: IExplorerLinkTarget): string => {
    const fragment = new URLSearchParams({policy: target.policyId, log: target.log})
    if (target.find !== undefined) fragment.set("find", target.find)
    if (target.entry !== undefined) fragment.set("entry", String(target.entry))
    return `${url.split("#")[0]}#${fragment.toString()}`
}
