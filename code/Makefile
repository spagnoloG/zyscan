
start:
	@echo "Starting rust app"
	@cargo run

start_dev:
	@echo "Starting rust app in dev mode"
	@cargo-watch -x run

start_db:
	@echo "Checking if mongodb container exists"
	$(eval CONTAINER_EXISTS := $(shell docker ps -a | grep mongodb_r))
	@if [ -n "$(CONTAINER_EXISTS)" ]; then \
		echo "Starting mongodb container"; \
		docker start mongodb_r; \
	else \
		echo "Creating mongodb container"; \
		docker run -d -p 27017:27017 --name mongodb_r mongo; \
	fi

lint:
	@echo "Linting code"
	#@cargo clippy --fix ./src/*.rs
	@black ./src_py/*.py

stop_db:
	@echo "Stopping the database"
	docker stop mongodb_r
