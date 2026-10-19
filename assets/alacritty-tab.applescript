on run argv
    set targetDir to item 1 of argv
    tell application "Alacritty" to activate
    tell application "System Events"
        if not UI elements enabled then error "Enable FinderJump in Privacy & Security > Accessibility"
        tell process "Alacritty"
            set frontmost to true
            keystroke "t" using command down
            delay 0.4
            if not frontmost then error "Alacritty lost focus; cancelled directory change"
            keystroke "cd -- " & quoted form of targetDir
            key code 36
        end tell
    end tell
end run
