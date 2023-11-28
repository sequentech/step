#!/bin/sh

# First argument is the environment
ENVIRONMENT=${1-:dev}

if [ "$ENVIRONMENT" = "prod" ];then
    yarn start
else
    yarn --pure-lockfile --non-interactive
    yarn build:ui-essentials
    yarn start
fi