.DEFAULT_GOAL := build

# Local variables
THEMES = $(notdir $(basename $(wildcard etc/defaults/themes/*.yaml)))
SCREENSHOT_SAMPLE = prometheus.log

# Exported variables
export RUST_BACKTRACE=1

## Print help
help:
	@echo "$$(tput setaf 2)Usage$$(tput sgr0)";sed -ne"/^## /{h;s/.*//;:d" -e"H;n;s/^## /---/;td" -e"s/:.*//;G;s/\\n## /===/;s/\\n//g;p;}" ${MAKEFILE_LIST}|awk -F === -v n=$$(tput cols) -v i=4 -v a="$$(tput setaf 6)" -v z="$$(tput sgr0)" '{printf"  '$$(tput setaf 2)make$$(tput sgr0)' %s%s%s\t",a,$$1,z;m=split($$2,w,"---");l=n-i;for(j=1;j<=m;j++){l-=length(w[j])+1;if(l<= 0){l=n-i-length(w[j])-1;}printf"%*s%s\n",-i," ",w[j];}}' | column -ts $$'\t'
.PHONY: help

## Build debug target
build:
	@cargo build --benches
.PHONY: build

## Build release target
build-release:
	@cargo build --release --locked
.PHONY: build-release

## Install binary
install:
	@cargo install --path . --locked
.PHONY: install

## Run tests
test:
	@cargo test
.PHONY: test

## Run benchmarks
bench:
	@cargo bench
.PHONY: bench

## Show usage of the binary
usage: build
	@env -i ./target/debug/hl --help
.PHONY: usage

## Clean build artifacts
clean:
	@cargo clean
.PHONY: clean

## Create screenshots
screenshots: build $(THEMES:%=screenshot-%)

screenshot-%: build
	@defaults write org.alacritty NSRequiresAquaSystemAppearance -bool yes
	@contrib/bin/screenshot.sh light $(SCREENSHOT_SAMPLE) $*
	@defaults write org.alacritty NSRequiresAquaSystemAppearance -bool no
	@contrib/bin/screenshot.sh dark $(SCREENSHOT_SAMPLE) $*
	@defaults delete org.alacritty NSRequiresAquaSystemAppearance
.PHONY: screenshot-%

## Install dependencies needed for contribution
contrib:
	@$(SHELL) contrib/bin/setup.sh
.PHONY: contrib

## Collect coverage
coverage:
	@cargo tarpaulin \
		--skip-clean \
		--workspace \
		--locked \
		--out Lcov \
		--output-dir target/coverage \
		--target-dir target/coverage \
		--exclude-files 'src/*_capnp.rs'
