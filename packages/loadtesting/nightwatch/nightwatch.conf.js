// SPDX-FileCopyrightText: 2025 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
module.exports = {
  src_folders: ["src"],

  webdriver: {
    start_process: true,
    server_path: "./node_modules/chromedriver/lib/chromedriver/chromedriver"
  },

  test_settings: {
    default: {
      desiredCapabilities: {
        browserName: "chrome",
        'goog:chromeOptions': {
          args: [
            '--no-sandbox',
            '--headless',//'--headless=new',
            '--disable-dev-shm-usage',
            '--disable-gpu',
            '--window-size=500,700'//,
            //'--disable-web-security',
            //'--allow-running-insecure-content',
            //'--disable-features=VizDisplayCompositor'
          ]
        }
      },
    },

    // Non-headless environment for debugging
    chrome: {
      desiredCapabilities: {
        browserName: "chrome",
        'goog:chromeOptions': {
          args: [
            '--no-sandbox',
            '--disable-dev-shm-usage',
            '--window-size=1920,1080'
          ]
        }
      },
    },
  },
}
