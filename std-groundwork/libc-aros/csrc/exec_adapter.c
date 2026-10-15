/* Older v11 CRT serializes argv into a DOS command line, dropping empty argv
 * entries. Its vfork exec failure path can also lose errno during takeover.
 * Keep these adaptations at the SDK boundary; do not synthesize success. */
#include <errno.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

int rs_execvp(const char *file, char *const argv[])
{
    size_t count = 0;
    int has_empty = 0;
    char **copy = NULL;
    struct stat status;
    if (!file || !argv) { errno = EINVAL; return -1; }
    /* Match the SDK's qualified-name rule. Bare names retain its PATH search.
     * This check precedes takeover, where the selected CRT loses child errno.
     * It is not a replacement for checking the actual exec result (TOCTOU). */
    if (strchr(file, '/') || strchr(file, ':')) {
        if (stat(file, &status) != 0) return -1;
        if (S_ISDIR(status.st_mode) || !(status.st_mode & 0111)) {
            errno = EACCES;
            return -1;
        }
    }
    while (argv[count]) {
        if (count && !argv[count][0]) has_empty = 1;
        if (count >= SIZE_MAX / sizeof(*copy) - 1) { errno = E2BIG; return -1; }
        count++;
    }
    if (has_empty) {
        copy = malloc((count + 1) * sizeof(*copy));
        if (!copy) { errno = ENOMEM; return -1; }
        for (size_t i = 0; i < count; i++)
            copy[i] = (i && !argv[i][0]) ? (char *)"\"\"" : argv[i];
        copy[count] = NULL;
    }
    errno = 0;
    execvp(file, copy ? copy : argv);
    /* Successful exec never returns. A missing errno must remain a failure,
     * including failed PATH searches/races not caught by qualified preflight. */
    int error = errno ? errno : EIO;
    free(copy);
    errno = error;
    return -1;
}
