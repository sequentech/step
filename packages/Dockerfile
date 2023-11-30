FROM node:20-alpine

WORKDIR /usr/src/app/
EXPOSE 3000

# Install security updates and Yarn
RUN apk update && \
    apk upgrade && \
    apk add --no-cache yarn

COPY . .

ARG ENVIRONMENT=dev

# Install dependencies & Build the application for production
RUN if [ "$ENVIRONMENT" = "prod" ]; then \
        yarn --pure-lockfile --non-interactive; \
        yarn build:ui-essentials; \
    fi


ENTRYPOINT [ "/usr/src/app/init.sh", "start", "$ENVIRONMENT" ]

# # === Runtime Stage === #
# TODO: Make this service multi staged, 
#       The problem is that it's hard to copy the dists directory of each package since there are a lot of packages and we don't want to duplicate the COPY command
# FROM node:20-alpine

# WORKDIR /usr/src/app

# COPY --from=frontend-builder /usr/src/app/node_modules ./node_modules
# COPY --from=frontend-builder /usr/src/app/dist ./dist

# EXPOSE 3000
# # Use a non-root user for security
# USER node

# ENTRYPOINT [ "yarn", "start" ]
# #CMD ["./init.sh", "start"]
