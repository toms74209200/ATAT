.PHONY: help

help: ## Display this help screen
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

.PHONY: clean

clean:	## Clean build artifacts
	cargo clean

.PHONY: check lint fmt

check: lint fmt	## Run lint and format check

lint:	## Run cargo clippy
	cargo clippy -- -D warnings

fmt:	## Run cargo format check
	cargo fmt -- --check

.PHONY: test test-small test-medium

test: test-small test-medium	## Run all tests

test-small:	## Run small tests (unit tests)
	cargo test --lib

test-medium: clean	## Run medium tests (acceptance tests)
	PRIVATE_KEY=`cat secret/*.pem` \
	CLIENT_ID=$${CLIENT_ID} \
	TEST_GITHUB_TARGET_OWNER=$${TEST_GITHUB_TARGET_OWNER} \
	cargo test --test acceptance -- -c 1

.PHONY: coverage

coverage:	## Generate test coverage report
	cargo llvm-cov --lib --ignore-filename-regex `(grep -Ev '^\\s*(#|$$)' scripts/coverage_ignore.txt | sed 's#^#src/#; s#$$#.rs#' | paste -sd '|' -)`

.PHONY: all

all: check coverage test-medium	## Run all checks and tests
