// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
const { config: baseConfig } = require("./base.conf.js");

const parallelConfig = {
  maxInstances: 10,
  commonCapabilities: {
    build: 'browserstack-build-5',
    project: 'strand',
    // "browserstack.networkLogs": 'true',
  },
  capabilities: [
    {
    browserName: 'chrome',
    browserVersion: 'latest',
    "browserstack.networkLogs": 'true',
    os: 'Windows',
    osVersion: '11'
  }, {
    browserName: 'chrome',
    device: 'Samsung Galaxy S20'
  }
  ],
  specs: [
    './tests/specs/test.js'
  ],
};

exports.config = { ...baseConfig, ...parallelConfig };

// Code to support common capabilities
exports.config.capabilities.forEach(function (caps) {
  for (var i in exports.config.commonCapabilities) caps[i] = caps[i] || exports.config.commonCapabilities[i];
});
