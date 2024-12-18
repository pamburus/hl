#/bin/bash

set -e

ALACRITTY=/Applications/Alacritty.app/Contents/MacOS/alacritty
HL_SRC=$(cd $(dirname ${0:?})/../.. && pwd)
MODE=${1:?}
SAMPLE=${2:?}
THEME=${3:?}
TITLE="hl ${SAMPLE:?}"

"${ALACRITTY:?}" \
    --config-file "${HL_SRC:?}"/contrib/etc/alacritty/${MODE:?}.toml \
    -T "${TITLE:?}" \
    --hold \
    -e \
        "${HL_SRC:?}"/target/debug/hl \
        --config - \
        --theme ${THEME:?} \
        -P \
        "${HL_SRC:?}"/sample/${SAMPLE:?} &

sleep 0.5

mkdir -p "${HL_SRC:?}"/extra/screenshot/${THEME:?}
screencapture -l$(GetWindowID Alacritty "${TITLE:?}") "${HL_SRC:?}"/extra/screenshot/${THEME:?}/${MODE:?}.png

kill $!
