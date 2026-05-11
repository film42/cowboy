# Stage 1: Build the React frontend
FROM node:22-alpine AS frontend
WORKDIR /app/client
COPY client/package.json client/package-lock.json ./
RUN npm ci
COPY client/ ./
RUN npm run build

# Stage 2: Build the Rust server
FROM rust:1.95-alpine AS backend
RUN apk add --no-cache musl-dev
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/
RUN cargo build --release

# Stage 3: Runtime
FROM alpine:3.21
RUN apk add --no-cache ca-certificates
WORKDIR /app
COPY --from=backend /app/target/release/cowboy ./
COPY --from=frontend /app/client/dist ./client/dist/

ENV LIVEKIT_URL=ws://localhost:7880

EXPOSE 3000
CMD ["./cowboy"]
