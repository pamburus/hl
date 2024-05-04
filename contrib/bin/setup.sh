#/bin/bash

set -e

if [ ! -x "$(command -v /Applications/Alacritty.app/Contents/MacOS/alacritty)" ]; then
    echo "Please install alacritty first"
    echo "See https://github.com/alacritty/alacritty"
    exit 1
fi

if [ ! -x "$(command -v brew)" ]; then
    echo "Please install homebrew first"
    echo "See https://brew.sh"
    exit 1
fi

if [ ! -x "$(command -v GetWindowID)" ]; then
    brew install smokris/getwindowid/getwindowid
fi

if [ ! -x "$(command -v rustfilt)" ]; then
    cargo install rustfilt
fi

rustup component add llvm-tools-preview
