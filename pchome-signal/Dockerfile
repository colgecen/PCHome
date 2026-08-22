# ---- build stage ----
FROM rust:latest AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
COPY src ./src
RUN cargo build --release

# ---- runtime stage ----
FROM debian:bookworm-slim
WORKDIR /app
COPY --from=build /app/target/release/pchome-signal /app/pchome-signal
ENV PORT=8080
EXPOSE 8080
CMD ["/app/pchome-signal"]
