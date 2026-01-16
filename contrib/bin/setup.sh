#/bin/bash

set -euo pipefail

while :; do
    case $1 in
        -h|-\?|--help)
            echo "Usage: $0 [options] <setup>..."
            echo "Options:"
            echo "  --help  Display this help message"
            echo "Setups:"
            echo "  audit"
            echo "  bat"
            echo "  build"
            echo "  cargo-edit"
            echo "  coverage"
            echo "  gh"
            echo "  git-cliff"
            echo "  clippy"
            echo "  outdated"
            echo "  schema"
            echo "  screenshots"
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

setup_mdbook() {
    setup_cargo
    if [ ! cargo set-version --help >/dev/null 2>&1 ]; then
        cargo install mdbook
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
    if [ ! -x "$(command -v cargo-audit)" ]; then
        if [ -x "$(command -v brew)" ]; then
            brew install cargo-audit
        elif [ -x "$(command -v apk)" ]; then
            sudo apk add cargo-audit
        elif [ -x "$(command -v pacman)" ]; then
            sudo pacman -S cargo-audit
        elif [ -x "$(command -v cargo)" ]; then
            cargo install cargo-audit
        else
            echo "Please install cargo-audit manually"
        fi
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

setup_tombi() {
    if [ ! -x "$(command -v tombi)" ]; then
        if [ -x "$(command -v brew)" ]; then
            brew install tombi
        elif [ -x "$(command -v uv)" ]; then
            uv add --dev tombi
        else
            echo "Please install tombi manually"
        fi
    fi
}

setup_gh() {
    if [ ! -x "$(command -v gh)" ]; then
        if [ -x "$(command -v brew)" ]; then
            brew install gh
        elif [ -x "$(command -v apt-get)" ]; then
            sudo apt-get install gh
        elif [ -x "$(command -v yum)" ]; then
            sudo yum install gh
        elif [ -x "$(command -v pacman)" ]; then
            sudo pacman -S gh
        else
            echo "Please install gh manually"
        fi
    fi
}

setup_git_cliff() {
    if [ ! -x "$(command -v git-cliff)" ]; then
        if [ -x "$(command -v brew)" ]; then
            brew install git-cliff
        elif [ -x "$(command -v apt-get)" ]; then
            sudo apt-get install git-cliff
        elif [ -x "$(command -v yum)" ]; then
            sudo yum install git-cliff
        elif [ -x "$(command -v pacman)" ]; then
            sudo pacman -S git-cliff
        else
            setup_cargo
            cargo install git-cliff --locked
        fi
    fi
}

setup_bat() {
    if [ ! -x "$(command -v bat)" ]; then
        if [ -x "$(command -v brew)" ]; then
            brew install bat
        elif [ -x "$(command -v apt-get)" ]; then
            sudo apt-get install bat
        elif [ -x "$(command -v yum)" ]; then
            sudo yum install bat
        elif [ -x "$(command -v pacman)" ]; then
            sudo pacman -S bat
        else
            setup_cargo
            cargo install bat --locked
        fi
    fi
}

setup_markdownlint() {
    if [ ! -x "$(command -v markdownlint-cli2)" ]; then
        if [ -x "$(command -v brew)" ]; then
            brew install markdownlint-cli2
        elif [ -x "$(command -v npm)" ]; then
            npm install markdownlint-cli2 --global
        else
            echo "Please install markdownlint-cli2 manually"
        fi
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
            setup_tombi
            setup_taplo
            ;;
        coverage)
            setup_coverage_tools
            ;;
        screenshots)
            setup_screenshot_tools
            ;;
        clippy)
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
        mdbook)
            setup_mdbook
            ;;
        git-cliff)
            setup_git_cliff
            ;;
        gh)
            setup_gh
            ;;
        bat)
            setup_bat
            ;;
        markdown-lint)
            setup_markdownlint
            ;;
        *)
            echo "Unknown setup $1"
            exit 1
            ;;
    esac
    shift
done
