/* Parse only the Shell's freshly written numeric exit-code file. */
#ifndef RUNTIME_RECEIPT_CODE_H
#define RUNTIME_RECEIPT_CODE_H
#include <stdio.h>
#include <stdlib.h>
#include <ctype.h>
#include <errno.h>
static int read_exit_code(const char *path, char code[32]) {
    FILE *file = fopen(path, "r");
    if (!file) return 20;
    char text[64];
    size_t size = fread(text, 1, sizeof text - 1, file);
    int invalid = ferror(file) || !feof(file);
    fclose(file);
    if (invalid || !size) return 20;
    text[size] = 0;
    char *end;
    errno = 0;
    long value = strtol(text, &end, 10);
    if (errno || end == text || value < 0 || value > 255) return 20;
    while (isspace((unsigned char)*end)) ++end;
    if (*end || end != text + size) return 20;
    snprintf(code, 32, "%ld", value);
    return 0;
}
#endif
