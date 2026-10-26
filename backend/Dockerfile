FROM rust:1-slim AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY migrations ./migrations
COPY src ./src
RUN cargo build --release

FROM debian:stable-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build /app/target/release/toeic-backend /usr/local/bin/toeic-backend
ENV BIND_ADDR=0.0.0.0:3000 \
    DATABASE_URL=sqlite:///data/toeic.db?mode=rwc \
    CONTENT_DIR=/content
EXPOSE 3000
CMD ["toeic-backend"]
