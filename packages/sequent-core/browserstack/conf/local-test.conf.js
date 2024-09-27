// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
const { config: baseConfig } = require("./base.conf.js");

const localConfig = {
  // Adding browserstackLocal to browserstack-service to initiate local binary
  services: [['@browserstack/wdio-browserstack-service', { browserstackLocal: true, forcelocal: false }]],
  maxInstances: 5,
  commonCapabilities: {
    build: 'browserstack-build-' + process.env.GIT_COMMIT_SHA,
    project: 'sequent-core',
    // "browserstack.networkLogs": 'true',
  },
  capabilities: [],
  specs: [
    './tests/specs/local_test.js'
  ],
};

// https://www.browserstack.com/automate/capabilities
// https://www.browserstack.com/list-of-browsers-and-platforms/automate

var win_ = [
  // FAIL {'browserName': 'Chrome', 'browser_version': 'latest', 'os': 'Windows', 'os_version': '11'},
  {'browserName': 'Chrome', 'browser_version': 'latest', 'os': 'Windows', 'os_version': '10'},
  {'browserName': 'Firefox', 'browser_version': 'latest', 'os': 'Windows', 'os_version': '11'},
  {'browserName': 'Firefox', 'browser_version': 'latest', 'os': 'Windows', 'os_version': '10'},
  // FAIL {'browserName': 'Edge', 'browser_version': 'latest', 'os': 'Windows', 'os_version': '11'},
  {'browserName': 'Edge', 'browser_version': 'latest', 'os': 'Windows', 'os_version': '10'},
];

var osx_ = [
  {'browserName': 'Chrome', 'browser_version': 'latest', 'os': 'OS X', 'os_version': 'Monterey'},
  {'browserName': 'Chrome', 'browser_version': 'latest', 'os': 'OS X', 'os_version': 'Big Sur'},
  // FAIL {{'browserName': 'Safari', 'browser_version': 'latest', 'os': 'OS X', 'os_version': 'Monterey'},
  // FAIL {{'browserName': 'Safari', 'browser_version': 'latest', 'os': 'OS X ', 'os_version': 'Big Sur'},
  {'browserName': 'Firefox', 'browser_version': 'latest', 'os': 'OS X', 'os_version': 'Monterey'},
  {'browserName': 'Firefox', 'browser_version': 'latest', 'os': 'OS X ', 'os_version': 'Big Sur'},
  {'browserName': 'Edge', 'browser_version': 'latest', 'os': 'OS X', 'os_version': 'Monterey'},
  {'browserName': 'Edge', 'browser_version': 'latest', 'os': 'OS X ', 'os_version': 'Big Sur'},
];

var ipad = [
  { device: 'iPad Air 4', os_version: 14, browserName: 'Safari',},
  { device: 'iPad 9th', os_version: 15, browserName: 'Safari',},
  { device: 'iPad Pro 12.9 2021', os_version: 14, browserName: 'Safari',},
  { device: 'iPad Pro 12.9 2020', os_version: 14, browserName: 'Safari',},
  { device: 'iPad Pro 11 2021', os_version: 14, browserName: 'Safari',},
  { device: 'iPad Pro 12.9 2020', os_version: 13, browserName: 'Safari',},
  { device: 'iPad Mini 2021', os_version: 15, browserName: 'Safari',},
  { device: 'iPad Pro 12.9 2018', os_version: 15, browserName: 'Safari',},
  { device: 'iPad 8th', os_version: 14, browserName: 'Safari',},
  // FAIL { device: 'iPad Pro 12.9 2018', os_version: 13, browserName: 'Safari',},
  { device: 'iPad Pro 11 2020', os_version: 13.6, browserName: 'Safari',},
  { device: 'iPad Mini 2019', os_version: 13.6, browserName: 'Safari',},
  { device: 'iPad Air 2019', os_version: 13.6, browserName: 'Safari',},
  // FAIL { device: 'iPad 7th', os_version: 13, browserName: 'Safari',},
];

var iphone = [
  { device: 'iPhone 13', os_version: 15, browserName: 'Safari',},
  { device: 'iPhone 13 Pro Max', os_version: 15, browserName: 'Safari',},
  { device: 'iPhone 13 Pro', os_version: 15, browserName: 'Safari',},
  { device: 'iPhone 13 Mini', os_version: 15, browserName: 'Safari',},
  { device: 'iPhone 11', os_version: 15, browserName: 'Safari',},
  { device: 'iPhone 11 Pro', os_version: 15, browserName: 'Safari',},
  { device: 'iPhone XS', os_version: 14, browserName: 'Safari',},
  { device: 'iPhone 12', os_version: 14, browserName: 'Safari',},
  { device: 'iPhone 12 Pro Max', os_version: 14, browserName: 'Safari',},
  { device: 'iPhone 12 Pro', os_version: 14, browserName: 'Safari',},
  { device: 'iPhone 12 Mini', os_version: 14, browserName: 'Safari',},
  { device: 'iPhone 11', os_version: 14, browserName: 'Safari',},
  { device: 'iPhone 11 Pro Max', os_version: 14, browserName: 'Safari',},
  { device: 'iPhone 8', os_version: 15, browserName: 'Safari',},
  // FAIL { device: 'iPhone 7', os_version: 12, browserName: 'Safari',},
  { device: 'iPhone SE 2020', os_version: 13, browserName: 'Safari',},
];

var samsung = [
  { device: 'Samsung Galaxy S22 Ultra', os_version: 12.0, browserName: 'samsung',},
  { device: 'Samsung Galaxy S22 Plus', os_version: 12.0, browserName: 'samsung',},
  { device: 'Samsung Galaxy S22', os_version: 12.0, browserName: 'samsung',},
  { device: 'Samsung Galaxy S21', os_version: 12.0, browserName: 'samsung',},

  { device: 'Samsung Galaxy S21 Ultra', os_version: 11.0, browserName: 'samsung',},
  { device: 'Samsung Galaxy S21', os_version: 11.0, browserName: 'samsung',},
  { device: 'Samsung Galaxy S21 Plus', os_version: 11.0, browserName: 'samsung',},

  { device: 'Samsung Galaxy S20', os_version: 10.0, browserName: 'samsung',},
  { device: 'Samsung Galaxy S20 Plus', os_version: 10.0, browserName: 'samsung',},
  { device: 'Samsung Galaxy S20 Ultra', os_version: 10.0, browserName: 'samsung',},
  { device: 'Samsung Galaxy Note 20 Ultra', os_version: 10.0, browserName: 'samsung',},
  { device: 'Samsung Galaxy Note 20', os_version: 10.0, browserName: 'samsung',},
  { device: 'Samsung Galaxy A51', os_version: 10.0, browserName: 'samsung',},
  // FAIL { device: 'Samsung Galaxy A11', os_version: 10.0, browserName: 'android',},

  { device: 'Samsung Galaxy S22 Ultra', os_version: 12.0, browserName: 'chrome',},
  { device: 'Samsung Galaxy S22 Plus', os_version: 12.0, browserName: 'chrome',},
  { device: 'Samsung Galaxy S22', os_version: 12.0, browserName: 'chrome',},
  { device: 'Samsung Galaxy S21', os_version: 12.0, browserName: 'chrome',},

  { device: 'Samsung Galaxy S21 Ultra', os_version: 11.0, browserName: 'chrome',},
  { device: 'Samsung Galaxy S21', os_version: 11.0, browserName: 'chrome',},
  { device: 'Samsung Galaxy S21 Plus', os_version: 11.0, browserName: 'chrome',},

  { device: 'Samsung Galaxy S20', os_version: 10.0, browserName: 'chrome',},
  { device: 'Samsung Galaxy S20 Plus', os_version: 10.0, browserName: 'chrome',},
  { device: 'Samsung Galaxy S20 Ultra', os_version: 10.0, browserName: 'chrome',},
  { device: 'Samsung Galaxy Note 20 Ultra', os_version: 10.0, browserName: 'chrome',},
  { device: 'Samsung Galaxy Note 20', os_version: 10.0, browserName: 'chrome',},
  { device: 'Samsung Galaxy A51', os_version: 10.0, browserName: 'chrome',},
  // FAIL { device: 'Samsung Galaxy A11', os_version: 10.0, browserName: 'chrome',},
];

var google = [
  { device: 'Google Pixel 6 Pro', os_version: "13 Beta", browserName: 'chrome',},
  { device: 'Google Pixel 6 Pro', os_version: 12.0, browserName: 'chrome',},
  { device: 'Google Pixel 6', os_version: 12.0, browserName: 'chrome',},
  { device: 'Google Pixel 5', os_version: 12.0, browserName: 'chrome',},

  { device: 'Google Pixel 5', os_version: 11.0, browserName: 'chrome',},
  { device: 'Google Pixel 4', os_version: 11.0, browserName: 'chrome',},
];

var win = ext(win_, 0);
var osx = ext(osx_, 0);

localConfig.capabilities.push(...win);
/*localConfig.capabilities.push(...osx);
localConfig.capabilities.push(...iphone);
localConfig.capabilities.push(...ipad);
localConfig.capabilities.push(...samsung);
localConfig.capabilities.push(...google);*/

exports.config = { ...baseConfig, ...localConfig };

// Code to support common capabilities
exports.config.capabilities.forEach(function (caps) {
  for (var i in exports.config.commonCapabilities) caps[i] = caps[i] || exports.config.commonCapabilities[i];
});

function ext(caps, size) {
  var out = [];
  caps.forEach(function (cap) {
    out.push(cap);
    for(let i = 1; i < size + 1; i++) {
      var c = {...cap};
      c.browser_version = 'latest-' + i;
      out.push(c);
    }
  })

  return out;
}