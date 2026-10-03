// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import type {Locator, Page} from "@playwright/test"
import {
    ballotDesignFixture,
    DESIGN_COLORS,
    DESIGN_NAMES,
    type BallotDesignOptions,
} from "@sequentech/ui-test-kit/fixtures/ballot-design"
import {test, expect, eventPath, type Portal} from "./fixtures"

const PNG = Uint8Array.from(
    atob(
        "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg=="
    ),
    (char) => char.charCodeAt(0)
)
const PHONE = {width: 390, height: 844}

type Design = ReturnType<typeof ballotDesignFixture>["design"]

function publish(portal: Portal, options: BallotDesignOptions = {}) {
    const logoUrl = portal.s3.putBytes("public", "design/logo.png", PNG, "image/png")
    const fixture = ballotDesignFixture({logoUrl, ...options})
    for (const key of fixture.design.pictures) portal.s3.putBytes("public", key, PNG, "image/png")
    portal.data = fixture
    portal.publish()
    return {design: fixture.design, logoUrl}
}

const openList = (page: Page, portal: Portal, language = "en") =>
    page.goto(`${portal.origin}${eventPath}?lang=${language}`)

async function openBallot(page: Page, portal: Portal, language = "en") {
    await openList(page, portal, language)
    await page.locator(".click-to-vote-button").click()
    await page.locator(".start-voting-button").click()
    await expect(page.locator(".contest-container").first()).toBeVisible()
}

const contestTitles = (page: Page) => page.locator(".contest-title-text").allTextContents()

const candidateNames = (contest: Locator) => contest.locator(".candidate-title").allTextContents()

/** How many columns the candidates of a contest are laid out in. */
const columns = (contest: Locator) =>
    contest
        .locator(".candidate-item")
        .evaluateAll(
            (items) => new Set(items.map((item) => Math.round(item.getBoundingClientRect().left))).size
        )

const expectPublishedFrame = (page: Page) =>
    expect(page.locator(".app-root")).toHaveCSS("border-top-color", DESIGN_COLORS.frame)

test("the ballot list is drawn with the published stylesheet, logo and names", async ({
    page,
    portal,
}) => {
    const {logoUrl} = publish(portal)
    await openList(page, portal)
    await expect(page.getByRole("heading", {name: DESIGN_NAMES.election.en})).toBeVisible()
    await expect(page.locator("img.header-logo")).toHaveAttribute("src", logoUrl)
    await expectPublishedFrame(page)
})

test("the ballot shows the published contests, candidates, columns and pictures", async ({
    page,
    portal,
}) => {
    const {design, logoUrl} = publish(portal)
    await openBallot(page, portal)
    expect(await contestTitles(page)).toEqual(design.contests)

    const [senator, partyList] = await page.locator(".contest-container").all()
    expect(await candidateNames(senator)).toEqual(design.senators)
    expect(await candidateNames(partyList)).toEqual(design.parties)
    expect(await columns(senator)).toBe(design.columns.senator)
    expect(await columns(partyList)).toBe(design.columns.partyList)

    const pictures = await page
        .locator(".candidate-item img")
        .evaluateAll((images) => images.map((image) => (image as HTMLImageElement).src))
    expect(pictures).toEqual(
        [...design.senators, ...design.parties].map((name) =>
            expect.stringContaining(design.pictures.find((key) => key.includes(slug(name)))!)
        )
    )

    await expect(page.locator("img.header-logo")).toHaveAttribute("src", logoUrl)
    await expectPublishedFrame(page)
    await expect(page.locator(".contest-title").first()).toHaveCSS(
        "color",
        DESIGN_COLORS.contestTitle
    )
})

test("the review screen lists the published contests and the voter's marks", async ({
    page,
    portal,
}) => {
    const {design} = publish(portal)
    await openBallot(page, portal)
    await page.getByRole("checkbox", {name: design.senators[0]}).check()
    await page.getByRole("checkbox", {name: design.parties[1]}).check()
    await page.getByRole("button", {name: "Next", exact: true}).click()
    await expect(page.locator(".review-screen")).toBeVisible()
    expect(await contestTitles(page)).toEqual(design.contests)
    await expect(page.locator(".review-screen").getByText(design.senators[0])).toBeVisible()
    await expect(page.locator(".review-screen").getByText(design.parties[1])).toBeVisible()
    await expectPublishedFrame(page)
})

test("the published design is shown in each of its languages", async ({page, portal}) => {
    publish(portal)
    await openList(page, portal, "tl")
    await expect(page.getByRole("heading", {name: DESIGN_NAMES.election.tl})).toBeVisible()
    await openBallot(page, portal, "tl")
    expect(await contestTitles(page)).toEqual([
        DESIGN_NAMES.senator.tl,
        DESIGN_NAMES.partyList.tl,
    ])
})

test.describe("on a phone", () => {
    test.use({viewport: PHONE})

    test("the ballot keeps the published order in one column", async ({page, portal}) => {
        const {design} = publish(portal)
        await openList(page, portal)
        await expectPublishedFrame(page)
        await openBallot(page, portal)
        expect(await contestTitles(page)).toEqual(design.contests)
        const [senator, partyList] = await page.locator(".contest-container").all()
        expect(await candidateNames(senator)).toEqual(design.senators)
        expect(await columns(senator)).toBe(1)
        expect(await columns(partyList)).toBe(1)
        expect(
            await page.evaluate(
                () => document.documentElement.scrollWidth <= document.documentElement.clientWidth
            )
        ).toBe(true)
    })
})

test("a random contest order is random for the voter and kept on the review screen", async ({
    page,
    portal,
}) => {
    const {design} = publish(portal, {contestsOrder: "random", localContests: 4})
    const seen: string[][] = []
    for (let visit = 0; visit < 3; visit++) {
        await openBallot(page, portal)
        const shown = await contestTitles(page)
        expect([...shown].sort()).toEqual([...design.contests].sort())

        // The order holds while the voter marks the ballot.
        await page.getByRole("checkbox", {name: design.parties[0]}).check()
        expect(await contestTitles(page)).toEqual(shown)

        await page.getByRole("button", {name: "Next", exact: true}).click()
        await expect(page.locator(".review-screen")).toBeVisible()
        expect(await contestTitles(page)).toEqual(shown)

        seen.push(shown)
        await page.evaluate(() => sessionStorage.clear())
    }
    // Six contests have 720 orders: three visits all in the published order is not chance.
    expect(seen.some((shown) => shown.join() !== design.contests.join())).toBe(true)
})

function slug(name: string) {
    return name.toLowerCase().replace(/\W+/g, "-")
}
