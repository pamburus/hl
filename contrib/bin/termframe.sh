#!/bin/bash

if command -v termframe &> /dev/null; then
    TERMFRAME=$(command -v termframe)
else
    cargo install --locked --git https://github.com/pamburus/termframe
    TERMFRAME=termframe
fi

${TERMFRAME:?} "$@"
