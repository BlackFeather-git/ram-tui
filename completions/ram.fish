# Fish completion for ram-tui

set -l themes default dracula catppuccin nord tokyo-night gruvbox cyberpunk rose-pine everforest kanagawa monokai solarized monochrome
set -l symbols block braille

complete -c ram -s r -l rate -d "Refresh interval in ms (20-2000)" -x
complete -c ram -s n -l count -d "Number of top processes (1-10000)" -x
complete -c ram -s 1 -l once -d "Output one snapshot and exit"
complete -c ram -l json -d "Output machine-readable JSON snapshot"
complete -c ram -l no-group -d "Show individual PIDs instead of grouping"
complete -c ram -l compact -d "Compact mode: meters only"
complete -c ram -l mini -d "Mini mode: usage bar + percentage only"
complete -c ram -l tiny -d "Tiny mode: single-line status bar format"
complete -c ram -l theme -d "Color theme" -x -a "$themes"
complete -c ram -l symbol -d "Meter graph style" -x -a "$symbols"
complete -c ram -l update -d "Update ram-tui to latest release in-place and exit"
complete -c ram -l force -d "Force in-place update even if installed via package manager"
complete -c ram -l check-update -d "Check for updates and exit"
complete -c ram -l no-update-check -d "Disable background update check"
complete -c ram -s h -l help -d "Show help and options"
complete -c ram -s v -l version -d "Show version"
