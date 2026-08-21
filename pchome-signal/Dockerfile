# Stage 1: Build
FROM golang:1.23-alpine AS builder

RUN apk add --no-cache git ca-certificates

WORKDIR /app

COPY go.mod go.sum ./
RUN go mod download

COPY . .

RUN CGO_ENABLED=0 GOOS=linux go build -ldflags="-s -w" -o /pchome-signal .

# Stage 2: Runtime
FROM alpine:3.19

RUN apk --no-cache add ca-certificates tzdata openssl

WORKDIR /app

COPY --from=builder /pchome-signal .
COPY docker-entrypoint.sh /usr/local/bin/

RUN chmod +x /usr/local/bin/docker-entrypoint.sh

EXPOSE 8080

ENTRYPOINT ["docker-entrypoint.sh"]
CMD ["./pchome-signal", "-tls-cert=/certs/tls.crt", "-tls-key=/certs/tls.key"]
