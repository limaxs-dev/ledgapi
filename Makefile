SHELL := /usr/bin/env bash
.DEFAULT_GOAL := help

CARGO     ?= cargo
ARCHAVEN  ?= archaven

.PHONY: help
help: ## Show this help
	@awk 'BEGIN {FS = ":.*## "} /^[a-zA-Z_-]+:.*## / {printf "  %-18s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

.PHONY: fmt
fmt: ## cargo fmt --all
	$(CARGO) fmt --all

.PHONY: fmt-check
fmt-check: ## cargo fmt --all -- --check
	$(CARGO) fmt --all -- --check

.PHONY: clippy
clippy: ## cargo clippy with -D warnings
	$(CARGO) clippy --workspace --all-targets --all-features -- -D warnings

.PHONY: test
test: ## cargo test --all-features
	$(CARGO) test --all-features

.PHONY: architecture
architecture: ## module-boundary smoke
	$(CARGO) test -p ledgapi --test architecture

.PHONY: deny
deny: ## cargo deny check
	$(CARGO) deny check

.PHONY: archaven
archaven: ## archaven check
	$(ARCHAVEN) check

.PHONY: ci
ci: fmt-check clippy test architecture deny archaven ## full local CI surrogate
