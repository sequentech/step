#!/bin/sh

yarn --pure-lockfile --non-interactive
yarn build:ui-essentials
yarn start