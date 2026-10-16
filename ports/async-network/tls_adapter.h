/* Opaque, owner-task-only stepped TLS ABI; no Mbed TLS layout crosses FFI. */
#ifndef RS_TLS_ADAPTER_H
#define RS_TLS_ADAPTER_H
#include <stddef.h>
#include <stdint.h>
struct rs_tls;
struct rs_tls_error { int32_t domain, code; uint32_t verify; int32_t os_error; char message[192]; };
/* Callback returns byte count or negative errno; called only during a step. */
typedef int (*rs_tls_bio)(void *, unsigned char *, size_t, int writing);
enum { RS_TLS_OK=0, RS_TLS_READ=1, RS_TLS_WRITE=2, RS_TLS_AGAIN=3, RS_TLS_EOF=4, RS_TLS_ERROR=-1 };
struct rs_tls *rs_tls_new(const unsigned char *, size_t, const char *, rs_tls_bio, void *, struct rs_tls_error *);
int rs_tls_handshake(struct rs_tls *);
int rs_tls_read(struct rs_tls *, unsigned char *, size_t, size_t *);
int rs_tls_write(struct rs_tls *, const unsigned char *, size_t, size_t *);
int rs_tls_close(struct rs_tls *);
int rs_tls_error(struct rs_tls *, struct rs_tls_error *);
int rs_tls_free(struct rs_tls *);
#endif
