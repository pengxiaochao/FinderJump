on run argv
    set targetDir to item 1 of argv
    set alreadyRunning to application id "com.googlecode.iterm2" is running
    tell application id "com.googlecode.iterm2"
        activate
        if (count of windows) = 0 then
            set targetWindow to (create window with default profile)
            set targetSession to current session of targetWindow
        else if alreadyRunning then
            tell current window
                set targetTab to (create tab with default profile)
                set targetSession to current session of targetTab
            end tell
        else
            set targetSession to current session of current window
        end if
        tell targetSession to write text ("cd -- " & quoted form of targetDir)
    end tell
end run
