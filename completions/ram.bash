#!/usr/bin/env bash
# Bash completion for ram-tui

_ram_completions() {
    local cur prev opts themes symbols
    COMPREPLY=()
    cur="${COMPWORDS[COMPCWORD]}"
    prev="${COMPWORDS[COMPCWORD-1]}"
    opts="--theme --symbol --rate -r --count -n --compact --mini --tiny --json --once -1 --no-group --update --force --check-update --no-update-check --help -h --version -v"
    themes="default dracula catppuccin nord tokyo-night gruvbox cyberpunk rose-pine everforest kanagawa monokai solarized monochrome"
    symbols="block braille"

    case "${prev}" in
        --theme)
            COMPREPLY=( $(compgen -W "${themes}" -- "${cur}") )
            return 0
            ;;
        --symbol)
            COMPREPLY=( $(compgen -W "${symbols}" -- "${cur}") )
            return 0
            ;;
        -r|--rate|-n|--count)
            return 0
            ;;
    esac

    if [[ ${cur} == -* ]]; then
        COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
        return 0
    fi
}
complete -F _ram_completions ram
