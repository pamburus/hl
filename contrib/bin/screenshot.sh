#/bin/bash

set -e

HL_SRC=$(cd $(dirname ${0:?})/../.. && pwd)
MODE=${1:?}
SAMPLE=${2:?}
THEME=${3:?}
TITLE="hl sample/${SAMPLE:?}"

mkdir -p "${HL_SRC:?}"/extra/screenshot/${THEME:?}

"${HL_SRC:?}"/contrib/bin/termframe.sh \
    --title "${TITLE:?}" \
    --mode ${MODE:?} \
    --embed-fonts true \
    -W 120 \
    -H 23 \
    --font-family "JetBrains Mono, Fira Code, Cascadia Code, Source Code Pro, Consolas, Menlo, Monaco, DejaVu Sans Mono, monospace" \
    --font-size 12 \
    -o "${HL_SRC:?}"/extra/screenshot/${THEME:?}/${MODE:?}.svg \
    -- \
    "${HL_SRC:?}"/target/debug/hl \
        --config - \
        --theme ${THEME:?} \
        --time-format '%T' \
        -P \
        "${HL_SRC:?}"/sample/${SAMPLE:?}
