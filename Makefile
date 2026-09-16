# devpit
#
# Seven targets, and each one does the whole job it names. The Tauri CLI finds
# its project by searching subfolders of the working directory, so every target
# runs from the repository root.

TAURI := ./node_modules/.bin/tauri

# The tests get a home of their own.
#
# `Store::root` is `dirs::home_dir()/.devpit`, so a suite run against the real
# HOME opened — and migrated — the store someone actually uses. The XDG
# directories go with it, because a cache written to `~/.cache` is the same
# mistake one directory over.
#
# `CARGO_HOME` and `RUSTUP_HOME` stay where they are. They are keyed to HOME
# too, and moving them would download the toolchain and every crate again on
# each run.
TEST_HOME := $(CURDIR)/target/test-home
CARGO_HOME ?= $(HOME)/.cargo
RUSTUP_HOME ?= $(HOME)/.rustup
TEST_ENV := HOME=$(TEST_HOME) \
	XDG_CACHE_HOME=$(TEST_HOME)/.cache \
	XDG_CONFIG_HOME=$(TEST_HOME)/.config \
	XDG_DATA_HOME=$(TEST_HOME)/.local/share \
	CARGO_HOME=$(CARGO_HOME) \
	RUSTUP_HOME=$(RUSTUP_HOME)

.DEFAULT_GOAL := help
.PHONY: help setup dev build test fmt clean

help: ## Show this help
	@grep -hE '^[a-z-]+:.*?## ' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-8s\033[0m %s\n", $$1, $$2}'

setup: ## Install dependencies
	pnpm install

dev: node_modules ## Run the app with hot reload
	$(TAURI) dev

build: node_modules ## Bundle a release binary, frontend included
	$(TAURI) build

# A .deb only. The AppImage bundler downloads linuxdeploy and its plugins at
# bundle time, so a build that includes it needs the network and fails behind
# anything that blocks those hosts. Ask for it explicitly when you want one:
#
#     ./node_modules/.bin/tauri build --bundles appimage

test: node_modules ## Everything CI runs: guards, Rust, frontend
	cargo fmt --all --check
	cargo xtask check
	cargo clippy --workspace --all-targets -- -D warnings
	@rm -rf $(TEST_HOME)
	@mkdir -p $(TEST_HOME)/.cache $(TEST_HOME)/.config $(TEST_HOME)/.local/share
	@touch $(TEST_HOME)/.zshenv $(TEST_HOME)/.zshrc $(TEST_HOME)/.bashrc $(TEST_HOME)/.profile
	$(TEST_ENV) cargo test --workspace
	pnpm --filter ./web test
	pnpm --filter ./web build
	@echo "\nall green"

fmt: ## Format the tree
	cargo fmt --all

clean: ## Remove build output
	cargo clean
	rm -rf web/dist

# Targets needing JS deps depend on this, so a fresh clone fails with the
# install rather than with "tauri: not found".
node_modules: package.json pnpm-lock.yaml
	pnpm install
	@touch node_modules
