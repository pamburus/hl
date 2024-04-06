#/bin/bash

set -e

ALACRITTY=/Applications/Alacritty.app/Contents/MacOS/alacritty
HL_SRC=$(cd $(dirname ${0:?})/../.. && pwd)
MODE=${1:?}
SAMPLE=${2:?}
TITLE="hl ${SAMPLE:?}"

HL_CONFIG= "${ALACRITTY:?}" \
    --config-file "${HL_SRC:?}"/contrib/etc/alacritty/${MODE:?}.toml \
    -T "${TITLE:?}" \
    --hold \
    -e \
        "${HL_SRC:?}"/target/debug/hl \
        -P \
        "${HL_SRC:?}"/sample/${SAMPLE:?} &

sleep 1

screencapture -l$(GetWindowID Alacritty "${TITLE:?}") "${HL_SRC:?}"/doc/screenshot-${MODE:?}.png

kill $!
