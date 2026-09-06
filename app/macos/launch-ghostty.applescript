on run arguments
    set herdrPath to item 1 of arguments
    set homePath to item 2 of arguments
    set launchCommand to "exec /usr/bin/env -u HERDR_ENV -u HERDR_BIN_PATH -u HERDR_PANE_ID -u HERDR_SOCKET_PATH -u HERDR_CLIENT_SOCKET_PATH -u HERDR_STARTUP_CWD -u HERDR_TAB_ID -u HERDR_WORKSPACE_ID HERDR_CLIENT_TOP_PADDING=1 " & quoted form of herdrPath & linefeed

    tell application "Ghostty"
        -- Create the requested window before activation; activating the app
        -- first can also trigger its ordinary empty startup window.
        set surfaceConfig to new surface configuration
        set initial working directory of surfaceConfig to homePath
        set initial input of surfaceConfig to launchCommand
        set herdrWindow to new window with configuration surfaceConfig
        activate window herdrWindow
        perform action "toggle_fullscreen" on (focused terminal of selected tab of herdrWindow)
    end tell
end run
