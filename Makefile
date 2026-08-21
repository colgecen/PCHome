.PHONY: help run run-docker test test-desktop test-signal test-mobile build clean

help:
	@echo "PChome - Available commands:"
	@echo "  make run           - Run signal server + desktop daemon"
	@echo "  make run-docker    - Run signal server via Docker"
	@echo "  make test          - Run all tests"
	@echo "  make test-desktop  - Run desktop tests"
	@echo "  make test-signal   - Run signal tests"
	@echo "  make build         - Build all modules"
	@echo "  make clean         - Clean build artifacts"

run: build
	@echo "Starting PChome..."
	@echo "Signal server: http://localhost:8080"
	@echo "Desktop HUD:   file://$(shell pwd)/pchome-desktop/src-ui/index.html"
	@./scripts/run-local.sh

run-docker:
	@echo "Starting signal server via Docker..."
	cd pchome-signal && docker compose up -d
	@echo "Signal server: https://localhost:8443"

test: test-desktop test-signal
	@echo "All tests passed"

test-desktop:
	cd pchome-desktop && cargo test --all-targets --all-features

test-signal:
	cd pchome-signal && go test ./...

test-mobile:
	cd pchome-mobile && ./gradlew test

build:
	@echo "Building all modules..."
	cd pchome-signal && go mod tidy && go build ./...
	cd pchome-desktop && cargo build
	cd pchome-mobile && ./gradlew assembleDebug

clean:
	cd pchome-desktop && cargo clean
	cd pchome-signal && rm -f pchome-signal
	cd pchome-mobile && ./gradlew clean
