.PHONY: help build test run docker-build docker-up docker-down docker-logs clean

help:
	@echo "LoxBerry Rust - Available Commands:"
	@echo ""
	@echo "  make build         - Build all Rust crates"
	@echo "  make test          - Run all tests"
	@echo "  make run           - Run daemon locally"
	@echo "  make docker-build  - Build Docker image"
	@echo "  make docker-up     - Start Docker containers"
	@echo "  make docker-down   - Stop Docker containers"
	@echo "  make docker-logs   - View Docker logs"
	@echo "  make clean         - Clean build artifacts"
	@echo ""

build:
	cargo build --release

test:
	cargo test

run:
	LBHOMEDIR=$(PWD)/volumes cargo run --bin loxberry-daemon

docker-build:
	docker-compose build

docker-up:
	docker-compose up -d

docker-down:
	docker-compose down

docker-logs:
	docker-compose logs -f

clean:
	cargo clean
	rm -rf target/
