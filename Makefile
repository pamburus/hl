.DEFAULT_GOAL := build

# Local variables
THEMES = $(notdir $(basename $(wildcard etc/defaults/themes/*.yaml)))
SCREENSHOT_SAMPLE = prometheus.log

# Exported variables
export RUST_BACKTRACE=1

# The list of files that are intentionally ignored while being tracked
ignored-tracked-files = .vscode/settings.json

## Print help
help:
	@echo "$$(tput setaf 2)Usage$$(tput sgr0)";sed -ne"/^## /{h;s/.*//;:d" -e"H;n;s/^## /---/;td" -e"s/:.*//;G;s/\\n## /===/;s/\\n//g;p;}" ${MAKEFILE_LIST}|awk -F === -v n=$$(tput cols) -v i=4 -v a="$$(tput setaf 6)" -v z="$$(tput sgr0)" '{printf"  '$$(tput setaf 2)make$$(tput sgr0)' %s%s%s\t",a,$$1,z;m=split($$2,w,"---");l=n-i;for(j=1;j<=m;j++){l-=length(w[j])+1;if(l<= 0){l=n-i-length(w[j])-1;}printf"%*s%s\n",-i," ",w[j];}}' | column -ts $$'\t'
.PHONY: help

## Run continuous integration tests
ci: check-fmt test build
	@cargo run -- --version
.PHONY: ci

## Run code formatting tests
check-fmt:
	@cargo +nightly fmt --all -- --check
.PHONY: check-fmt

## Automatically format code
fmt:
	@cargo +nightly fmt --all
.PHONY: fmt

## Build debug target
build: contrib-build
	@cargo build --benches
.PHONY: build

## Build release target
build-release: contrib-build
	@cargo build --release --locked
.PHONY: build-release

## Install binary and man pages
install: contrib-build install-man-pages
	@cargo install --path . --locked
.PHONY: install

## Install man pages
install-man-pages: ~/share/man/man1/hl.1
	@echo $$(tput setaf 3)NOTE:$$(tput sgr0) ensure $$(tput setaf 2)~/share/man$$(tput sgr0) is added to $$(tput setaf 2)MANPATH$$(tput sgr0) environment variable
.PHONY: install-man-pages

~/share/man/man1/hl.1: contrib-build | ~/share/man/man1
	@HL_CONFIG= cargo run --release --locked -- --man-page >$@

~/share/man/man1:
	@mkdir -p $@

## Install versioned binary
install-versioned: contrib-build
	@cargo install --path . --locked
	@cp ${HOME}/.cargo/bin/hl ${HOME}/.cargo/bin/$$(${HOME}/.cargo/bin/hl --version | tr ' ' '-')
.PHONY: install-versioned

## Run tests
test: contrib-build
	@cargo test --workspace
.PHONY: test

## Run benchmarks
bench: contrib-build
	@cargo bench --workspace
.PHONY: bench

## Show usage of the binary
usage: build
	@env -i HL_CONFIG= ./target/debug/hl --help
.PHONY: usage

## Clean build artifacts
clean: contrib-build
	@cargo clean
.PHONY: clean

## Create screenshots
screenshots: build $(THEMES:%=screenshot-%)
.PHONY: screenshots

screenshot-%: build contrib-screenshots
	@defaults write org.alacritty NSRequiresAquaSystemAppearance -bool yes
	@contrib/bin/screenshot.sh light $(SCREENSHOT_SAMPLE) $*
	@defaults write org.alacritty NSRequiresAquaSystemAppearance -bool no
	@contrib/bin/screenshot.sh dark $(SCREENSHOT_SAMPLE) $*
	@defaults delete org.alacritty NSRequiresAquaSystemAppearance
.PHONY: screenshot-%

## Collect coverage
coverage:
	@contrib/bin/setup.sh coverage
	@build/ci/coverage.sh
.PHONY: coverage

## Skip ignored tracked files
skip-ignored:
	@git update-index --skip-worktree $(ignored-tracked-files)
.PHONY: skip-ignored

## Undo skip-ignored
no-skip-ignored:
	@git update-index --no-skip-worktree $(ignored-tracked-files)
.PHONY: no-skip-ignored

contrib-build:
	@contrib/bin/setup.sh build
.PHONY: contrib-build

contrib-screenshots:
	@contrib/bin/setup.sh screenshots
.PHONY: contrib-screenshots
