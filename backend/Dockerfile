FROM rust:1.82-slim AS builder

WORKDIR /usr/src/app

# Install build dependencies including OpenSSL dev packages and pkg-config
RUN apt-get update && \
  apt-get install -y \
  pkg-config \
  libssl-dev \
  libpq-dev \
  && rm -rf /var/lib/apt/lists/*

# Copy the project files
COPY . .

RUN echo $PWD
# Install the package
RUN cargo build --release

RUN ls -la target/release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libpq5 openssl && rm -rf /var/lib/apt/lists/*

# Copy the binary from the target/release directory instead
COPY --from=builder /usr/src/app/target/release/ttt_backend /usr/local/bin/ttt_backend

EXPOSE 8000
CMD ["ttt_backend"]