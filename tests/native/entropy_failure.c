/* Compile the actual adapter with AROS_JENT_FORCE_FAIL; no fallback allowed. */
#include <assert.h>
#include <errno.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>
extern int getentropy(void *, size_t);
int main(void)
{
    unsigned char bytes[32];
    for (int attempt = 0; attempt < 3; attempt++) {
        memset(bytes, 0xa5, sizeof bytes);
        int rc = getentropy(bytes, sizeof bytes);
        assert(rc == -1 && errno == EIO);
        for (size_t i = 0; i < sizeof bytes; i++) assert(bytes[i] == 0);
    }
    puts("ENTROPY_FAILURE_OK: init failure returned EIO and wiped output");
    return 0;
}
