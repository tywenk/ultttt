FROM node:20-alpine AS builder

ARG VITE_API_BASE_URL=https://api.ultimatetictactoemmo.com
ARG VITE_API_WS_BASE_URL=wss://api.ultimatetictactoemmo.com

RUN npm i -g pnpm
WORKDIR /app
COPY package.json pnpm-lock.yaml ./
RUN pnpm i --frozen-lockfile
COPY . .
RUN pnpm build

FROM node:20-alpine AS production
WORKDIR /app
COPY --from=builder /app/build ./build
COPY package.json pnpm-lock.yaml ./
RUN npm i -g pnpm serve && pnpm i --prod --frozen-lockfile

EXPOSE 3000
CMD ["serve", "-s", "build/client", "-l", "tcp://0.0.0.0:3000"]
