FROM node:20-alpine

# Install Yarn
RUN apk add --no-cache yarn

# WORKDIR /usr/src/app/test-app/
# COPY test-app/package*.json ./
# COPY test-app/yarn.lock ./
# RUN yarn install --pure-lockfile --non-interactive

# Install dependencies
WORKDIR /usr/src/app/
COPY package*.json ./
COPY yarn.lock ./
#RUN yarn  --pure-lockfile --non-interactive

EXPOSE 3000

CMD ["yarn", "start"]