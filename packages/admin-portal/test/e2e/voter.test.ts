import { ExtendDescribeThis, NightwatchAPI } from "nightwatch"
import { voterDetails } from "..";

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

// eslint-disable-next-line jest/valid-describe-callback
describe("voters tests", function (this: ExtendDescribeThis<LoginThis>) {
	before(function (this: ExtendDescribeThis<LoginThis>, browser) {
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

	it("create a voter", async (browser: NightwatchAPI) => {
		const resultElement = await browser.element.findAll(
			`a[title = '${createElectionEvent.config.electionEventName}']`
		)
		resultElement[resultElement.length - 1].click()

		browser.assert.visible("a.election-event-voter-tab").click("a.election-event-voter-tab")

		browser.isPresent(
			{
				selector: "button.voter-add-button",
				suppressNotFoundErrors: true,
				timeout: 1000,
			},
			(result) => {
				if (result.value) {
					browser.assert
						.visible("button.voter-add-button")
						.click("button.voter-add-button")
				} else {
					browser.assert.visible("button.add-button").click("button.add-button")
				}
				browser
					.sendKeys("input[name=first_name]", voterDetails.firstName)
					.sendKeys("input[name=last_name]", voterDetails.lastName)
					.sendKeys("input[name=email]", voterDetails.email)
					.sendKeys("input[name=username]", voterDetails.username)
					.assert.enabled("button[type=submit]")
					.click("button[type=submit]")
					.pause(1000)
					.assert.textContains("span.first_name", voterDetails.firstName)
					.assert.textContains("span.last_name", voterDetails.lastName)
					.assert.textContains("span.email", voterDetails.email)
					.assert.textContains("span.username", voterDetails.username)
			}
		)
	})

	it("edit a voter to set password", async (browser: NightwatchAPI) => {
		const resultElement = await browser.element.findAll(
			`a[title = '${createElectionEvent.config.electionEventName}']`
		)
		resultElement[resultElement.length - 1].click()

		browser.assert.visible("a.election-event-voter-tab").click("a.election-event-voter-tab")

		browser.isPresent(
			{
				selector: "button.voter-add-button",
				suppressNotFoundErrors: true,
				timeout: 1000,
			},
			(result) => {
				if (result.value) {
					browser.end()
				} else {
					browser.assert.visible(".edit-voter-icon").click(".edit-voter-icon")
					browser
						.sendKeys("input[name=password]", "secretepassword")
						.sendKeys("input[name=repeat_password]", "secretepassword")
						.assert.enabled("button[type=submit]")
						.click("button[type=submit]")
						.pause(200)
						.assert.textContains("span.first_name", voterDetails.firstName)
				}
			}
		)
	})

	it("edit a voter to set area", async (browser: NightwatchAPI) => {
		// create area
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

		// activate voters tab
		const resultElement = await browser.element.findAll(
			`a[title = '${createElectionEvent.config.electionEventName}']`
		)
		resultElement[resultElement.length - 1].click()

		browser.assert.visible("a.election-event-voter-tab").click("a.election-event-voter-tab")

		browser.isPresent(
			{
				selector: "button.voter-add-button",
				suppressNotFoundErrors: true,
				timeout: 1000,
			},
			async (result) => {
				if (result.value) {
					browser.end()
				} else {
					browser.assert.visible(".edit-voter-icon").click(".edit-voter-icon")
					browser.assert.visible(".select-voter-area").click(".select-voter-area")
					const opcion = await browser.element.findByRole("option")
					opcion.click()
					browser.assert
						.enabled("button[type=submit]")
						.click("button[type=submit]")
						.pause(200)
						.assert.textContains("span.first_name", voterDetails.firstName)
				}
			}
		)

		// delete area
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
						.pause(1000)
						.assert.not.elementPresent("span.area-description")
				}
			}
		)
	})

	it("delete a voter", async (browser: NightwatchAPI) => {
		const resultElement = await browser.element.findAll(
			`a[title = '${createElectionEvent.config.electionEventName}']`
		)
		resultElement[resultElement.length - 1].click()

		browser.assert.visible("a.election-event-voter-tab").click("a.election-event-voter-tab")

		browser.isPresent(
			{
				selector: "button.voter-add-button",
				suppressNotFoundErrors: true,
				timeout: 1000,
			},
			(result) => {
				if (result.value) {
					browser.end()
				} else {
					browser.assert.visible(".delete-voter-icon").click(".delete-voter-icon")
					browser.assert
						.enabled(`button.ok-button`)
						.click("button.ok-button")
						.pause(1000)
						.assert.not.elementPresent("span.first_name")
				}
			}
		)
	})
})