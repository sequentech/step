#!/bin/sh

# First argument is the yarn action
ACTION=${1-:start}
# Second argument is the environment
ENVIRONMENT=${2-:dev}

if [ "$ENVIRONMENT" = "prod" ];then
    yarn $ACTION
else
    yarn --pure-lockfile --non-interactive
    yarn build:ui-essentials
    yarn $ACTION
fi
