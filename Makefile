.PHONY: all build start run bench sim test docker-build docker-up docker-down clean help

CARGO ?= cargo
CARGO_DIR ?= engine

all: build

help:
	@echo "SIH26123 AMR Fleet Coordination Engine"
	@echo ""
	@echo "Available commands:"
	@echo "  make start        - Build and run real-time web dashboard on port 3000"
	@echo "  make bench        - Run comparative benchmark against Centralized CBS"
	@echo "  make sim          - Run headless multi-robot simulation (4 robots, 8 tasks)"
	@echo "  make test         - Execute all 49 automated integration tests"
	@echo "  make build        - Compile release binary in engine/"
	@echo "  make docker-build - Build containerized Docker image"
	@echo "  make docker-up    - Run containerized application via Docker Compose"
	@echo "  make docker-down  - Stop Docker Compose container"
	@echo "  make clean        - Remove build artifacts"

build:
	cd $(CARGO_DIR) && $(CARGO) build --release

start: run

run:
	cd $(CARGO_DIR) && $(CARGO) run --release -- dashboard --port 3000 --robots 4 --tasks 8

bench:
	cd $(CARGO_DIR) && $(CARGO) run --release -- bench --width 15 --height 15 --tasks 5

sim:
	cd $(CARGO_DIR) && $(CARGO) run --release -- sim --robots 4 --width 15 --height 15 --tasks 8

test:
	cd $(CARGO_DIR) && $(CARGO) test

docker-build:
	docker compose build

docker-up:
	docker compose up -d
	@echo "Dashboard running at http://localhost:3000"

docker-down:
	docker compose down

clean:
	cd $(CARGO_DIR) && $(CARGO) clean
