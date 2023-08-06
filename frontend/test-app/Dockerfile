FROM node:20-alpine

# Install Yarn
RUN apk add --no-cache yarn

WORKDIR /usr/src/app/

COPY package*.json ./

RUN yarn install

EXPOSE 3000

CMD ["yarn", "start"]