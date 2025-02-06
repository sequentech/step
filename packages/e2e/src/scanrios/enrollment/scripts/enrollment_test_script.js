// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

client => {
    const testUrl = "{url}";
    const otpCode = "123456";
    const validIdInput = "*[id='sequent.read-only.id-card-type']";
    const passwordInput = "*[type='password'][id='password']";
    const confirmPasswordInput = "*[type='password'][id='password-confirm']";
    const emailInput = "*[type=text][id='email']";
    const embassyInput = "*[id='embassy']";
    const termsCheckbox = "*[type=checkbox][name='termsOfService']";
    const submitButton = "*[type=submit]";
    
    const otpContainer = "*[id='otp-inputs']"
    const finishButton = "*[id='loginContinueLink']"


      const enroll = (i) => {
            client.url(testUrl)
            .waitForElementVisible('body', 1000)
            .click(validIdInput)
            .waitForElementVisible(validIdInput + "option[value='Philippine Passport']", 2000)
            .click(validIdInput + " option[value='Philippine Passport']")
            .setValue(passwordInput, 'Pass12345*')
            .setValue(confirmPasswordInput, 'Pass12345*')
            .setValue(emailInput, `user${i}@gmail.com`) 
            .click(embassyInput)
            .waitForElementVisible(embassyInput + " option[value='Tokyo PE']", 2000)
            .click(embassyInput + " option[value='Tokyo PE']")
            .click(termsCheckbox)
            .takeScreenshot( `enrollment${i}.png`)
            .click(submitButton)


            client.waitForElementVisible(otpContainer,2000)
            for (let otpIndex = 1; otpIndex <= 6; otpIndex++) {
                  client.setValue(`*[id='otp-${otpIndex}']`, otpCode.at(otpIndex - 1));
            }
            client.takeScreenshot(`otp${i}.png`)
            .click(submitButton)

            client.waitForElementVisible(finishButton,10000)
            .takeScreenshot(`finisht_screen${i}.png`)
            .click(finishButton)
      }

      enroll(1);

    //   for(let i=1; i<=enrollmentsCount; i++) {
    //         try {
    //             console.log(`test ${i}`)
    //              enroll(i);
    //         } catch (error) {
    //             console.log(`Error ${error}\n`);
    //         }
    //   }

}