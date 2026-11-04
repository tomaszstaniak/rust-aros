/* Separate synchronous process, invoked before/after the probe by the Shell. */
#include <proto/exec.h>
#include <proto/bsdsocket.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <stdio.h>
#include <string.h>
#include "receipt_code.h"
struct Library *SocketBase;
int main(int argc, char **argv) {
    if (argc != 5) return 20;
    char code[32];
    const char *value = argv[4];
    if (!strcmp(argv[3], "end")) {
        if (read_exit_code(argv[4], code)) return 20;
        value = code;
    } else if (strcmp(argv[3], "start") || strcmp(argv[4], "0")) return 20;
    SocketBase = OpenLibrary("bsdsocket.library", 4);
    if (!SocketBase) return 20;
    int fd = socket(AF_INET, SOCK_STREAM, 0), rc = 20;
    if (fd < 0) goto done;
    struct sockaddr_in address; memset(&address, 0, sizeof address);
    address.sin_len = sizeof address; address.sin_family = AF_INET;
    address.sin_port = htons(18791); address.sin_addr.s_addr = htonl(0x0a000202);
    if (connect(fd, (struct sockaddr *)&address, sizeof address)) goto close;
    char message[256];
    int size = snprintf(message, sizeof message, "%s %s %s %s\n", argv[1], argv[2], argv[3], value);
    if (size < 0 || size >= sizeof message) goto close;
    for (int sent = 0; sent < size;) {
        int n = send(fd, message + sent, size - sent, 0);
        if (n <= 0) goto close;
        sent += n;
    }
    fd_set readset; FD_ZERO(&readset); FD_SET(fd, &readset);
    struct timeval timeout = {3, 0};
    if (WaitSelect(fd + 1, &readset, NULL, NULL, &timeout, NULL) > 0) {
        char reply[3]; int received = 0;
        while (received < 3) {
            int n = recv(fd, reply + received, 3 - received, 0);
            if (n <= 0) break;
            received += n;
        }
        if (received == 3 && !memcmp(reply, "OK\n", 3)) rc = 0;
    }
close: CloseSocket(fd);
done: CloseLibrary(SocketBase); return rc;
}
