// Reconstructed from the current app's resource-relative osascript launcher.
// No personal paths or global terminal configuration are required.
#include <libgen.h>
#include <mach-o/dyld.h>
#include <pwd.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

int main(void) {
    char executable[4096], resolved[4096], contents[4096];
    uint32_t size = sizeof(executable);
    if (_NSGetExecutablePath(executable, &size) != 0 ||
        realpath(executable, resolved) == NULL) {
        fputs("Could not locate Herdr.app\n", stderr);
        return 1;
    }
    char *macos = dirname(resolved);
    if (snprintf(contents, sizeof(contents), "%s", macos) >= (int)sizeof(contents)) return 1;
    char *base = dirname(contents);
    char binary[4096], script[4096];
    if (snprintf(binary, sizeof(binary), "%s/Resources/herdr", base) >= (int)sizeof(binary) ||
        snprintf(script, sizeof(script), "%s/Resources/launch-ghostty.scpt", base) >= (int)sizeof(script)) {
        fputs("Herdr resource path is too long\n", stderr);
        return 1;
    }
    const char *user_home = getenv("HOME");
    if (user_home == NULL || *user_home == '\0') {
        struct passwd *user = getpwuid(getuid());
        if (user == NULL || user->pw_dir == NULL) return 1;
        user_home = user->pw_dir;
    }
    execl("/usr/bin/osascript", "osascript", script, binary, user_home, (char *)NULL);
    perror("Could not launch Herdr");
    return 1;
}
