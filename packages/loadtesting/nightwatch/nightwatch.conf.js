module.exports = {
  src_folders: ["src"],

  webdriver: {
    server_path: "/nightwatch/node_modules/chromedriver/bin/chromedriver"
  },

  test_settings: {
    default: {
      desiredCapabilities: {
        browserName: "chrome",
        'goog:chromeOptions': {
          args: ['--no-sandbox']
        }
      },
    },
  },
}
