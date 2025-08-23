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
            '--headless=new',
            '--disable-dev-shm-usage',
            '--disable-gpu',
            '--window-size=1920,1080',
            '--disable-web-security',
            '--allow-running-insecure-content',
            '--disable-features=VizDisplayCompositor'
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
