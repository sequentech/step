FROM node:20-alpine

WORKDIR /usr/src/app/

# Install security updates and Yarn
RUN apk update && \
    apk upgrade && \
    apk add --no-cache yarn

COPY . .

ARG ENVIRONMENT=dev

# Install dependencies & Build the application for production
RUN if [ "$ENVIRONMENT" = "prod" ];then \
    yarn --pure-lockfile --non-interactive \
    yarn build:ui-essentials; \
    fi

EXPOSE 3000
USER node

ENTRYPOINT [ "/usr/src/app/init.sh", "$ENVIRONMENT" ]

# # === Runtime Stage === #
# FROM node:20-alpine

# WORKDIR /usr/src/app

# COPY --from=frontend-builder /usr/src/app/node_modules ./node_modules
# COPY --from=frontend-builder /usr/src/app/dist ./dist

# EXPOSE 3000
# # Use a non-root user for security
# USER node

# ENTRYPOINT [ "yarn", "start" ]
# #CMD ["./init.sh", "start"]