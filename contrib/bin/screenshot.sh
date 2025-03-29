#/bin/bash

set -e

HL_SRC=$(cd $(dirname ${0:?})/../.. && pwd)
MODE=${1:?}
SAMPLE=${2:?}
THEME=${3:?}
TERMFRAME_THEME=${4:-one-double}
TITLE="hl sample/${SAMPLE:?}"

mkdir -p "${HL_SRC:?}"/extra/screenshot/${THEME:?}

printf "generating screenshot for \e[1m%s\e[m sample and \e[1m%s\e[m mode with \e[1m%s\e[m theme" "${SAMPLE:?}" "${MODE:?}" "${THEME:?}"
if [ "${TERMFRAME_THEME}" != "one-double" ]; then
    printf " and \e[1m%s\e[m termframe theme" "${TERMFRAME_THEME:?}"
fi
printf "\n"

"${HL_SRC:?}"/contrib/bin/termframe.sh \
    --title "${TITLE:?}" \
    --mode ${MODE:?} \
    -W 120 \
    -H 23 \
    --theme ${TERMFRAME_THEME:?} \
    --font-family "JetBrains Mono, Fira Code, Cascadia Code, Source Code Pro, Consolas, Menlo, Monaco, DejaVu Sans Mono, monospace" \
    --font-size 12 \
    --embed-fonts true \
    -o "${HL_SRC:?}"/extra/screenshot/${THEME:?}/${MODE:?}.svg \
    -- \
    "${HL_SRC:?}"/target/debug/hl \
        --config - \
        --theme ${THEME:?} \
        --time-format '%T' \
        -P \
        "${HL_SRC:?}"/sample/${SAMPLE:?}
