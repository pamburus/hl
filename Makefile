# Common makefile helpers
include build/make/common.mk

.DEFAULT_GOAL := build
SHELL = /bin/bash

# Local variables
THEMES = $(notdir $(basename $(wildcard etc/defaults/themes/*.yaml)))
SCREENSHOT_SAMPLE = prometheus.log

# Exported variables
export RUST_BACKTRACE=full

# The list of files that are intentionally ignored while being tracked
ignored-tracked-files = .vscode/settings.json

## Build debug target
.PHONY: build
build: contrib-build
	@cargo build --benches

## Run continuous integration tests
.PHONY: ci
ci: check-fmt check-schema test build
	@cargo run -- --version

## Run code formatting tests
.PHONY: check-fmt
check-fmt: contrib-build-nightly
	@cargo +nightly fmt --all -- --check

## Run schema validation tests
.PHONY: check-schema
check-schema: contrib-schema
	@taplo check

## Automatically format code
.PHONY: fmt
fmt: contrib-build-nightly
	@cargo +nightly fmt --all

## Build release target
.PHONY: build-release
build-release: contrib-build
	@cargo build --release --locked

## Install binary and man pages
.PHONY: install
install: contrib-build install-man-pages
	@cargo install --path . --locked

## Install man pages
.PHONY: install-man-pages
install-man-pages: ~/share/man/man1/hl.1
	@echo $$(tput setaf 3)NOTE:$$(tput sgr0) ensure $$(tput setaf 2)~/share/man$$(tput sgr0) is added to $$(tput setaf 2)MANPATH$$(tput sgr0) environment variable

~/share/man/man1/hl.1: contrib-build | ~/share/man/man1
	cargo run --release --locked -- --config - --man-page >"$@"

~/share/man/man1:
	@mkdir -p "$@"

## Install versioned binary
.PHONY: install-versioned
install-versioned: contrib-build
	@cargo install --path . --locked
	@cp ${HOME}/.cargo/bin/hl ${HOME}/.cargo/bin/$$(${HOME}/.cargo/bin/hl --version | tr ' ' '-')

## Run tests
.PHONY: test
test: contrib-build
	@cargo test --workspace

## Run benchmarks
.PHONY: bench
bench: contrib-build
	@cargo bench --workspace

## Show usage of the binary
.PHONY: usage
usage: build
	@./target/debug/hl --config - --help

## Clean build artifacts
.PHONY: clean
clean: contrib-build
	@cargo clean

## Create screenshots
.PHONY: screenshots
screenshots: build $(THEMES:%=screenshot-%)

screenshot-%: build contrib-screenshots
	@defaults write org.alacritty NSRequiresAquaSystemAppearance -bool yes
	@$(SHELL) contrib/bin/screenshot.sh light $(SCREENSHOT_SAMPLE) $*
	@defaults write org.alacritty NSRequiresAquaSystemAppearance -bool no
	@$(SHELL) contrib/bin/screenshot.sh dark $(SCREENSHOT_SAMPLE) $*
	@defaults delete org.alacritty NSRequiresAquaSystemAppearance
.PHONY: screenshot-%

## Collect coverage
.PHONY: coverage
coverage: contrib-coverage
	@$(SHELL) contrib/bin/setup.sh coverage
	@$(SHELL) build/ci/coverage.sh

## Skip ignored tracked files
.PHONY: skip-ignored
skip-ignored:
	@git update-index --skip-worktree $(ignored-tracked-files)

## Undo skip-ignored
.PHONY: no-skip-ignored
no-skip-ignored:
	@git update-index --no-skip-worktree $(ignored-tracked-files)

.PHONY: contrib-build
contrib-build:
	@$(SHELL) contrib/bin/setup.sh build

.PHONY: contrib-build-nightly
contrib-build-nightly:
	@$(SHELL) contrib/bin/setup.sh build-nightly

.PHONY: contrib-coverage
contrib-coverage:
	@$(SHELL) contrib/bin/setup.sh coverage

.PHONY: contrib-schema
contrib-schema:
	@$(SHELL) contrib/bin/setup.sh schema

.PHONY: contrib-screenshots
contrib-screenshots:
	@$(SHELL) contrib/bin/setup.sh screenshots
