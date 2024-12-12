FROM node:20-alpine AS builder
RUN npm i -g pnpm
WORKDIR /app
COPY package.json pnpm-lock.yaml ./
RUN pnpm i --frozen-lockfile
COPY . .
RUN pnpm build

FROM node:20-alpine as production
WORKDIR /app
COPY --from=builder /app/build ./build
COPY package.json pnpm-lock.yaml ./
RUN npm i -g pnpm serve && pnpm i --prod --frozen-lockfile

EXPOSE 3000
CMD ["npx", "serve", "-s", "build/client", "-l", "0.0.0.0:3000"]
