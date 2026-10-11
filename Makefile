.PHONY: help run run-docker test test-desktop test-signal test-mobile build clean

help:
	@echo "PCHome - Available commands:"
	@echo "  make run           - Run signal server + desktop daemon"
	@echo "  make run-docker    - Run signal server via Docker"
	@echo "  make test          - Run all tests"
	@echo "  make test-desktop  - Run desktop tests"
	@echo "  make test-signal   - Run signal tests"
	@echo "  make build         - Build all modules"
	@echo "  make clean         - Clean build artifacts"

run: build
	@echo "Starting PCHome..."
	@echo "Signal server: ws://localhost:8080/ws"
	@echo "Desktop daemon: egui HUD (PIN + telemetry)"
	@./scripts/run-local.sh

run-docker:
	@echo "Starting signal server via Docker..."
	cd PCHome-signal && docker compose up -d
	@echo "Signal server: https://localhost:8443"

test: test-desktop test-signal
	@echo "All tests passed"

test-desktop:
	cd PCHome-desktop && cargo test --all-targets --all-features

test-signal:
	cd PCHome-signal && cargo test

test-mobile:
	cd PCHome-mobile && ./gradlew test

build:
	@echo "Building all modules..."
	cd PCHome-signal && cargo build --release
	cd PCHome-desktop && cargo build --release
	cd PCHome-mobile && ./gradlew assembleDebug

clean:
	cd PCHome-desktop && cargo clean
	cd PCHome-signal && cargo clean
	cd PCHome-mobile && ./gradlew clean
