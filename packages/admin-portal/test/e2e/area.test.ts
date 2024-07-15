import { ExtendDescribeThis, NightwatchAPI } from "nightwatch"
import { candidateLink } from "..";

const createElectionEvent = require("../commands/createElectionEvent");
const deleteElectionEvent = require("../commands/deleteElectionEvent");


interface LoginThis {
	testUrl: string
	username: string
	password: string
	submitButton: string
	electionEventLink: string
	electionLink: string
	contestLink: string
	candidateLink: string
}

describe("areas tests", function (this: ExtendDescribeThis<LoginThis>) {

	before(function (this: ExtendDescribeThis<LoginThis>, browser) {
		// login
		browser.login()

		// create election event
		createElectionEvent.createElectionEvent(browser);
		createElectionEvent.createElection(browser);
		createElectionEvent.createContest(browser);
		createElectionEvent.createCandidates(browser);
	})

	after(async function (this: ExtendDescribeThis<LoginThis>, browser) {
		//delete election event
		deleteElectionEvent.deleteCandidates(browser);
		deleteElectionEvent.deleteContest(browser);
		deleteElectionEvent.deleteElection(browser);
		deleteElectionEvent.deleteElectionEvent(browser);

		// Logout
		browser
			.logout()
	})

	it("create an area", async (browser: NightwatchAPI) => {
		const resultElement = await browser.element.findAll(
			`a[title = '${createElectionEvent.config.electionEventName}']`
		)
		resultElement[resultElement.length - 1].click()

		browser.assert.visible("a.election-event-area-tab").click("a.election-event-area-tab")

		browser.isPresent(
			{
				selector: "button.area-add-button",
				suppressNotFoundErrors: true,
				timeout: 1000,
			},
			(result) => {
				if (result.value) {
					browser.assert.visible("button.area-add-button").click("button.area-add-button")
				} else {
					browser.assert.visible("button.add-button").click("button.add-button")
				}
				browser
					.sendKeys("input[name=name]", "this is an area name")
					.assert.enabled("button[type=submit]")
					.click("button[type=submit]")
					.pause(200)
					.assert.textContains("span.area-name", "this is an area name")
			}
		)
	})

	it("edit an area", async (browser: NightwatchAPI) => {
		const resultElement = await browser.element.findAll(
			`a[title = '${createElectionEvent.config.electionEventName}']`
		)
		resultElement[resultElement.length - 1].click()

		browser.assert.visible("a.election-event-area-tab").click("a.election-event-area-tab")

		browser.isPresent(
			{
				selector: "button.area-add-button",
				suppressNotFoundErrors: true,
				timeout: 1000,
			},
			(result) => {
				if (result.value) {
					browser.end()
				} else {
					browser.assert.visible(".edit-area-icon").click(".edit-area-icon")
					browser
						.sendKeys("input[name=description]", "this is an area description")
						.assert.enabled("button[type=submit]")
						.click("button[type=submit]")
						.pause(200)
						.assert.textContains("span.area-description", "this is an area description")
				}
			}
		)
	})

	it("edit an area contest", async (browser: NightwatchAPI) => {
		const resultElement = await browser.element.findAll(
			`a[title = '${createElectionEvent.config.electionEventName}']`
		)
		resultElement[resultElement.length - 1].click()

		browser.assert.visible("a.election-event-area-tab").click("a.election-event-area-tab")

		browser.isPresent(
			{
				selector: "button.area-add-button",
				suppressNotFoundErrors: true,
				timeout: 1000,
			},
			(result) => {
				if (result.value) {
					browser.end()
				} else {
					browser.assert.visible(".edit-area-icon").click(".edit-area-icon")
					browser
						.click(".area-contest label")
						.assert.enabled("button[type=submit]")
						.click("button[type=submit]")
						.pause(200)
				}
			}
		)
	})

	it("edit an area contest unset contest", async (browser: NightwatchAPI) => {
		const resultElement = await browser.element.findAll(
			`a[title = '${createElectionEvent.config.electionEventName}']`
		)
		resultElement[resultElement.length - 1].click()

		browser.assert.visible("a.election-event-area-tab").click("a.election-event-area-tab")

		browser.isPresent(
			{
				selector: "button.area-add-button",
				suppressNotFoundErrors: true,
				timeout: 1000,
			},
			(result) => {
				if (result.value) {
					browser.end()
				} else {
					browser.assert.visible(".edit-area-icon").click(".edit-area-icon")
					browser
						.click(".area-contest label")
						.assert.enabled("button[type=submit]")
						.click("button[type=submit]")
						.pause(200)
				}
			}
		)
	})

	it("delete an area", async (browser: NightwatchAPI) => {
		const resultElement = await browser.element.findAll(
			`a[title = '${createElectionEvent.config.electionEventName}']`
		)
		resultElement[resultElement.length - 1].click()

		browser.assert.visible("a.election-event-area-tab").click("a.election-event-area-tab")

		browser.isPresent(
			{
				selector: "button.area-add-button",
				suppressNotFoundErrors: true,
				timeout: 1000,
			},
			(result) => {
				if (result.value) {
					browser.end()
				} else {
					browser.assert.visible(".delete-area-icon").click(".delete-area-icon")
					browser.assert
						.enabled(`button.ok-button`)
						.click("button.ok-button")
						.pause(200)
						.assert.not.elementPresent("span.area-description")
				}
			}
		)
	})
})