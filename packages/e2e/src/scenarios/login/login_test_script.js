// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

client => {
    const url = "{url}"
    const password = "{password}"
    const otpCode = "123456"
    const numberOfVotes = "{numberOfVoters}"
    const otpContainer = "*[id='otp-inputs']"
    const submitButton = "*[type=submit]";

    const login = (iteration) => {
        const randomNumber = Math.floor(Math.random() * parseInt(numberOfVotes)) + 1
        const username = `user${randomNumber}@gmail.com`
        console.log(`starting iteration ${iteration} for: ${username}`)
        const timestamp = new Date().toISOString();
        console.log(`Current Timestamp: ${timestamp}`);
        //Login
        client.url(url)
            .waitForElementVisible('body', 20e3)
            .setValue('input[name=username]', username)
            .setValue('input[name=password]', password)
            .click('#kc-login')
            .pause(500)
        //OTP
        client.waitForElementVisible(otpContainer, 2000)
        for (let otpIndex = 1; otpIndex <= 6; otpIndex++) {
            client.setValue(`*[id='otp-${otpIndex}']`, otpCode.at(otpIndex - 1));
        }
        //Log out
        client.click(submitButton)
            .pause(500)
            .click("button.logout-button")
            .pause(500)
            .waitForElementVisible("li.logout-button", 20e3)
            .click('li.logout-button')
            .pause(500)
            .click("button.ok-button")
            .pause(500)
            .perform(() => {
                console.log(`Completed iteration #${iteration}`);
                login(iteration + 1);
            })
    }
    login(1)

}
