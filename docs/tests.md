# Tests system

The system uses [Nightwatch](https://nightwatchjs.org/) for testing the code.

In order the system to work, tests must be runned locally (tests inside the codespace are under review)

The process to run tests locally are:

- clone the repository to a local folder
- run `cd packages && yarn && yarn build:ui-essentials` to install all dependencies
- cd to the portal to be tested
- open the codespace in visual studio code and run either admin-portal or voting-portal as usual
- from the local env run the tests with the command:
  - run all e2e tests: `./node_modules/.bin/nightwatch -t ./test/e2e/* --headless`
  - run specific e2e tests: `./node_modules/.bin/nightwatch -t ./test/e2e/FILE_TO_RUN.test.ts --headless`

# Output

Example of output after running tests:

```
[areas tests] Test Suite
────────────────────────────────────────────────
Selenium Manager binary found at /home/enric/Developer/backend-services/packages/node_modules/selenium-webdriver/bin/linux/selenium-manager
Driver path: /usr/bin/chromedriver
Browser path: /usr/bin/google-chrome
  Using: chrome (114.0.5735.133) on LINUX.

  ✔ Element <input[name=username]> was visible after 1466 milliseconds.
  ✔ Element <input[name=password]> was visible after 20 milliseconds.

  Running create an area:
───────────────────────────────────────────────────────────────────────────────────────────────────
  ✔ Testing if element <a.election-event-area-tab> is visible (450ms)
  ✔ Testing if element <button.area-add-button> is visible (18ms)
  ✔ Testing if element <button[type=submit]> is enabled (30ms)
  ✔ Testing if element <span.area-name> contains text 'this is an area name' (83ms)

  ✨ PASSED. 4 assertions. (1.435s)

  Running edit an area:
───────────────────────────────────────────────────────────────────────────────────────────────────
  ✔ Testing if element <a.election-event-area-tab> is visible (471ms)
  ✔ Testing if element <.edit-area-icon> is visible (27ms)
  ✔ Testing if element <button[type=submit]> is enabled (14ms)
  ✔ Testing if element <span.area-description> contains text 'this is an area description' (114ms)

  ✨ PASSED. 4 assertions. (3.119s)

  Running edit an area contest:
───────────────────────────────────────────────────────────────────────────────────────────────────
  ✔ Testing if element <a.election-event-area-tab> is visible (905ms)
  ✔ Testing if element <.edit-area-icon> is visible (16ms)
  ✔ Testing if element <button[type=submit]> is enabled (48ms)

  ✨ PASSED. 3 assertions. (2.807s)

  Running edit an area contest unset contest:
───────────────────────────────────────────────────────────────────────────────────────────────────
  ✔ Testing if element <a.election-event-area-tab> is visible (952ms)
  ✔ Testing if element <.edit-area-icon> is visible (25ms)
  ✔ Testing if element <button[type=submit]> is enabled (13ms)

  ✨ PASSED. 3 assertions. (3.366s)

  Running delete an area:
───────────────────────────────────────────────────────────────────────────────────────────────────
  ✔ Testing if element <a.election-event-area-tab> is visible (924ms)
  ✔ Testing if element <.delete-area-icon> is visible (24ms)
  ✔ Testing if element <button.ok-button> is enabled (16ms)
  ✔ Testing if element <span.area-description> is not present (31ms)

  ✨ PASSED. 4 assertions. (3.66s)

  ✨ PASSED. 20 total assertions (35.649s)
 Wrote HTML report file to: /home/enric/Developer/backend-services/packages/admin-portal/tests_output/nightwatch-html-report/index.html

```
