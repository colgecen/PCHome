# ---- build stage ----
# Runtime ile aynı Debian sürümünde derle: rust:latest yeni glibc'ye
# geçtiğinde bookworm runtime'da "GLIBC_x.y not found" ile çökmesin.
FROM rust:bookworm AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
COPY src ./src
RUN cargo build --release

# ---- runtime stage ----
FROM debian:bookworm-slim
WORKDIR /app
COPY --from=build /app/target/release/PCHome-signal /app/PCHome-signal
# Render PORT'u kendisi verir (varsayılan 10000). Yerelde kod 8080'e düşer.
EXPOSE 8080
CMD ["/app/PCHome-signal"]
