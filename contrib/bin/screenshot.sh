#/bin/bash

set -e

HL_SRC=$(cd $(dirname ${0:?})/../.. && pwd)
MODE=${1:?}
SAMPLE=${2:?}
THEME=${3:-all}
TERMFRAME_THEME=${4:-auto}
TITLE="hl sample/${SAMPLE:?}"

DEFAULT_TERMFRAME_THEME=one-double

function screenshot() {
    local THEME=${1:?}
    local TERMFRAME_THEME=${TERMFRAME_THEME:?}

    # Theme mappings
    if [ "${TERMFRAME_THEME}" == "auto" ]; then
        case "${THEME}" in
            ayu-dark-24)
                TERMFRAME_THEME=ayu
                ;;
            ayu-light-24)
                TERMFRAME_THEME=ayu-light
                ;;
            ayu-mirage-24)
                TERMFRAME_THEME=ayu-mirage
                ;;
            one-dark-24)
                TERMFRAME_THEME=one-half-dark
                ;;
            one-light-24)
                TERMFRAME_THEME=one-half-light
                ;;
            *)
                TERMFRAME_THEME="${DEFAULT_TERMFRAME_THEME:?}"
                ;;
        esac
    fi

    mkdir -p "${HL_SRC:?}"/extra/screenshot/${THEME:?}

    printf "generating screenshot for \e[1m%s\e[m sample and \e[1m%s\e[m mode with \e[1m%s\e[m theme" "${SAMPLE:?}" "${MODE:?}" "${THEME:?}"
    if [ "${TERMFRAME_THEME}" != "one-double" ]; then
        printf " and \e[1m%s\e[m termframe theme" "${TERMFRAME_THEME:?}"
    fi
    printf "\n"

    "${HL_SRC:?}"/contrib/bin/termframe.sh \
        --title "${TITLE:?}" \
        --mode ${MODE:?} \
        -W 122 \
        -H 23 \
        --theme ${TERMFRAME_THEME:?} \
        --font-family "JetBrains Mono, Fira Code, Cascadia Code, Source Code Pro, Consolas, Menlo, Monaco, DejaVu Sans Mono, monospace" \
        --faint-font-weight 200 \
        --faint-opacity 0.6 \
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
}

function screenshot_all() {
    get_themes | xargs -n1 | while read THEME; do
        screenshot ${THEME:?}
    done
}

function get_themes() {
    "${HL_SRC:?}"/target/debug/hl --list-themes=${MODE:?}
}

if [ "${THEME:?}" == "all" ]; then
    screenshot_all ${MODE:?} ${SAMPLE:?}
else
    screenshot ${THEME:?} ${TERMFRAME_THEME:?}
fi
