#/bin/bash

set -e

while :; do
    case $1 in
        -h|-\?|--help)
            echo "Usage: $0 [options] <setup>..."
            echo "Options:"
            echo "  --help  Display this help message"
            echo "Setups:"
            echo "  build"
            echo "  cargo-edit"
            echo "  coverage"
            echo "  schema"
            echo "  screenshots"
            echo "  lint"
            echo "  audit"
            echo "  outdated"
            exit 1
            ;;
        --)
            shift
            break
            ;;
        -*|--*)
            echo "Unknown option $1"
            exit 1
            ;;
        *)
            break
    esac
done

setup_homebrew() {
    if [ ! -x "$(command -v brew)" ]; then
        echo "Please install homebrew"
        echo "See https://brew.sh"
        exit 1
    fi
}

setup_jq() {
    if [ ! -x "$(command -v jq)" ]; then
        echo "Please install jq"
        echo "See https://stedolan.github.io/jq/"
        exit 1
    fi
}

setup_alacritty() {
    if [ ! -x "$(command -v /Applications/Alacritty.app/Contents/MacOS/alacritty)" ]; then
        echo "Please install alacritty"
        echo "See https://github.com/alacritty/alacritty"
        exit 1
    fi
}

setup_get_window_id() {
    setup_homebrew
    if [ ! -x "$(command -v GetWindowID)" ]; then
        brew install smokris/getwindowid/getwindowid
    fi
}

setup_termframe() {
    if [ ! -x "$(command -v termframe)" ]; then
        brew tap pamburus/homebrew-tap
        brew install termframe
    fi
}

rust_is_required() {
    echo "Please install rust"
    echo "See https://doc.rust-lang.org/cargo/getting-started/installation.html"
    exit 1
}

setup_cargo() {
    if [ ! -x "$(command -v cargo)" ]; then
        rust_is_required
    fi
}

setup_cargo_nightly() {
    setup_cargo
    if ! (rustup toolchain list | grep -q nightly); then
        echo installing nightly toolchain
        rustup toolchain install nightly
    fi
}

setup_cargo_edit() {
    setup_cargo
    if [ ! cargo set-version --help >/dev/null 2>&1 ]; then
        cargo install cargo-edit
    fi
}

setup_rustfilt() {
    setup_cargo
    if [ ! -x "$(command -v rustfilt)" ]; then
        cargo install rustfilt
    fi
}

setup_rustup() {
    if [ ! -x "$(command -v rustup)" ]; then
        echo "Please install rustup"
        echo "See https://rustup.rs"
        exit 1
    fi
}

setup_rustc() {
    if [ ! -x "$(command -v rustc)" ]; then
        rust_is_required
    fi
}

setup_sed() {
    if [ ! -x "$(command -v sed)" ]; then
        echo "Please install sed"
        echo "See https://www.gnu.org/software/sed/"
        exit 1
    fi

}

setup_llvm_profdata() {
    setup_rustup
    setup_rustc
    setup_sed
    if [ ! -x "$(command -v $(rustc --print sysroot)/lib/rustlib/$(rustc -vV | sed -n 's|host: ||p')/bin/llvm-profdata)" ]; then
        rustup component add llvm-tools-preview
    fi
}

setup_clippy() {
    setup_rustup
    if ! (rustup component list | grep -q 'clippy.*(installed)'); then
        rustup component add clippy
    fi
}

setup_cargo_audit() {
    setup_cargo
    if [ ! -x "$(command -v cargo-audit)" ]; then
        cargo install cargo-audit
    fi
}

setup_cargo_outdated() {
    setup_cargo
    if [ ! -x "$(command -v cargo-outdated)" ]; then
        cargo install cargo-outdated
    fi
}

setup_coverage_tools() {
    setup_llvm_profdata
    setup_rustfilt
    setup_jq
}

setup_screenshot_tools() {
    setup_termframe
}

setup_taplo() {
    setup_cargo
    if [ ! -x "$(command -v taplo)" ]; then
        cargo install taplo-cli --locked --features lsp
    fi
}

# --- main ---

while [ $# -gt 0 ]; do
    case $1 in
        build)
            setup_cargo
            ;;
        build-nightly)
            setup_cargo_nightly
            ;;
        schema)
            setup_taplo
            ;;
        coverage)
            setup_coverage_tools
            ;;
        screenshots)
            setup_screenshot_tools
            ;;
        lint)
            setup_clippy
            ;;
        audit)
            setup_cargo_audit
            ;;
        outdated)
            setup_cargo_outdated
            ;;
        cargo-edit)
            setup_cargo_edit
            ;;
        *)
            echo "Unknown setup $1"
            exit 1
            ;;
    esac
    shift
done
