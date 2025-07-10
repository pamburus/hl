#!/bin/bash

if command -v termframe &> /dev/null; then
    TERMFRAME=$(command -v termframe)
else
    TERMFRAME=cargo run --locked --release --git github.com/pamburus/termframe --
fi

${TERMFRAME:?} "$@"
