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
            '--headless',
            '--disable-dev-shm-usage',
            '--disable-gpu'
          ]
        }
      },
    },
  },
}
