/* One bounded Mbed TLS operation per step, on the BSD socket owner task.
 * Required verification and jitterentropy are inherited from pinned apkg.
 * Foreign-task use is refused; a Rust !Send/!Sync wrapper owns each context.
 */
#include "tls_adapter.h"
#include <mbedtls/ssl.h>
#include <mbedtls/entropy.h>
#include <mbedtls/ctr_drbg.h>
#include <mbedtls/error.h>
#include <mbedtls/net_sockets.h>
#include <mbedtls/psa_util.h>
#include <psa/crypto.h>
#include <proto/exec.h>
#include <errno.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
extern int rs_selector_owner(void);
extern int aros_jent_status(void);
extern void aros_jent_shutdown(void);
struct rs_tls {
    mbedtls_ssl_context ssl;
    mbedtls_ssl_config config;
    mbedtls_x509_crt ca;
    mbedtls_entropy_context entropy;
    mbedtls_ctr_drbg_context drbg;
    struct Task *owner;
    rs_tls_bio bio;
    void *user;
    int raw_eof, handshaken, failed;
    struct rs_tls_error error;
};
static unsigned live;
static struct Task *crypto_owner;
static int owner(struct rs_tls *ctx) { return ctx && ctx->owner == FindTask(NULL); }
static int fail(struct rs_tls *ctx, int domain, int code) {
    ctx->failed = 1;
    ctx->error.domain = domain; ctx->error.code = code;
    ctx->error.verify = domain == 2 ? mbedtls_ssl_get_verify_result(&ctx->ssl) : 0;
    mbedtls_strerror(code, ctx->error.message, sizeof(ctx->error.message));
    return RS_TLS_ERROR;
}
static int bio_io(struct rs_tls *ctx, unsigned char *buffer, size_t len, int writing) {
    int rc = ctx->bio(ctx->user, buffer, len, writing);
    if (rc >= 0) { if (!writing && rc == 0) ctx->raw_eof=1; return rc; }
    if (rc == -EAGAIN || rc == -EWOULDBLOCK || rc == -EINTR)
        return writing ? MBEDTLS_ERR_SSL_WANT_WRITE : MBEDTLS_ERR_SSL_WANT_READ;
    ctx->error.os_error = -rc;
    return writing ? MBEDTLS_ERR_NET_SEND_FAILED : MBEDTLS_ERR_NET_RECV_FAILED;
}
static int send_bytes(void *ctx, const unsigned char *b, size_t n) { return bio_io(ctx,(unsigned char *)b,n,1); }
static int recv_bytes(void *ctx, unsigned char *b, size_t n) { return bio_io(ctx,b,n,0); }
int rs_tls_free(struct rs_tls *ctx) {
    if (!ctx) return 0;
    if (!owner(ctx)) return RS_TLS_ERROR;
    /* No close_notify here: destruction must never perform network I/O. */
    mbedtls_ssl_free(&ctx->ssl); mbedtls_ssl_config_free(&ctx->config);
    mbedtls_x509_crt_free(&ctx->ca); mbedtls_ctr_drbg_free(&ctx->drbg);
    mbedtls_entropy_free(&ctx->entropy); free(ctx);
    if (--live == 0) { mbedtls_psa_crypto_free(); aros_jent_shutdown(); crypto_owner=NULL; }
    return 0;
}
struct rs_tls *rs_tls_new(const unsigned char *ca, size_t ca_len, const char *name,
                          rs_tls_bio bio, void *user, struct rs_tls_error *error) {
    struct rs_tls *ctx; int rc;
    memset(error,0,sizeof(*error));
    if (!ca || !ca_len || !name || !*name || !bio) { error->domain=6; return NULL; }
    /* Guard before reading any mutable owner-only TLS global. */
    if (rs_selector_owner() < 0 || (live && crypto_owner != FindTask(NULL))) {
        error->domain=4; return NULL;
    }
    ctx=calloc(1,sizeof(*ctx)); if (!ctx) { error->domain=6; error->os_error=ENOMEM; return NULL; }
    ctx->owner=FindTask(NULL); ctx->bio=bio; ctx->user=user;
    if (!live++) crypto_owner=ctx->owner;
    mbedtls_ssl_init(&ctx->ssl); mbedtls_ssl_config_init(&ctx->config);
    mbedtls_x509_crt_init(&ctx->ca); mbedtls_entropy_init(&ctx->entropy); mbedtls_ctr_drbg_init(&ctx->drbg);
    rc=psa_crypto_init(); if (rc) {
        fail(ctx,3,rc);
        /* PSA status codes are not Mbed TLS error codes. Preserve their domain. */
        snprintf(ctx->error.message,sizeof(ctx->error.message),"PSA crypto initialization failed; jitterentropy=%d",aros_jent_status());
        goto failed;
    }
    /* Seed before any connect; a failed source must never fall back. */
    rc=mbedtls_ctr_drbg_seed(&ctx->drbg,mbedtls_entropy_func,&ctx->entropy,
                           (const unsigned char *)"rust-aros-v11",13);
    if (rc) { fail(ctx,3,rc); goto failed; }
    rc=mbedtls_x509_crt_parse(&ctx->ca,ca,ca_len); if (rc) { fail(ctx,6,rc); goto failed; }
    rc=mbedtls_ssl_config_defaults(&ctx->config,MBEDTLS_SSL_IS_CLIENT,MBEDTLS_SSL_TRANSPORT_STREAM,MBEDTLS_SSL_PRESET_DEFAULT);
    if (rc) { fail(ctx,6,rc); goto failed; }
    mbedtls_ssl_conf_authmode(&ctx->config,MBEDTLS_SSL_VERIFY_REQUIRED);
    mbedtls_ssl_conf_ca_chain(&ctx->config,&ctx->ca,NULL);
    mbedtls_ssl_conf_rng(&ctx->config,mbedtls_ctr_drbg_random,&ctx->drbg);
    rc=mbedtls_ssl_setup(&ctx->ssl,&ctx->config); if (rc) { fail(ctx,6,rc); goto failed; }
    rc=mbedtls_ssl_set_hostname(&ctx->ssl,name); if (rc) { fail(ctx,6,rc); goto failed; }
    mbedtls_ssl_set_bio(&ctx->ssl,ctx,send_bytes,recv_bytes,NULL);
    return ctx;
failed:
    *error=ctx->error; rs_tls_free(ctx); return NULL;
}
static int status(struct rs_tls *ctx, int rc) {
    if (rc == MBEDTLS_ERR_SSL_WANT_READ) return RS_TLS_READ;
    if (rc == MBEDTLS_ERR_SSL_WANT_WRITE) return RS_TLS_WRITE;
    if (rc == MBEDTLS_ERR_SSL_RECEIVED_NEW_SESSION_TICKET) return RS_TLS_AGAIN;
    if (rc < 0) return fail(ctx,ctx->error.os_error ? 5 : (rc == MBEDTLS_ERR_X509_CERT_VERIFY_FAILED ? 2 : 1),rc);
    return RS_TLS_OK;
}
int rs_tls_handshake(struct rs_tls *ctx) {
    int rc;
    if (!owner(ctx) || ctx->failed) return RS_TLS_ERROR;
    if (ctx->handshaken) return RS_TLS_OK;
    rc=mbedtls_ssl_handshake_step(&ctx->ssl);
    if (!rc && !mbedtls_ssl_is_handshake_over(&ctx->ssl)) return RS_TLS_AGAIN;
    if (!rc) { if (mbedtls_ssl_get_verify_result(&ctx->ssl)) return fail(ctx,2,MBEDTLS_ERR_X509_CERT_VERIFY_FAILED); ctx->handshaken=1; }
    return status(ctx,rc);
}
int rs_tls_read(struct rs_tls *ctx, unsigned char *b, size_t n, size_t *got) {
    int rc; *got=0;
    if (!owner(ctx) || ctx->failed || !ctx->handshaken || !n) return RS_TLS_ERROR;
    rc=mbedtls_ssl_read(&ctx->ssl,b,n);
    if (rc == MBEDTLS_ERR_SSL_PEER_CLOSE_NOTIFY) return RS_TLS_EOF;
    /* TCP EOF is not authenticated TLS EOF, even after a complete HTTP body. */
    if (rc == 0 || ctx->raw_eof) return fail(ctx,5,MBEDTLS_ERR_SSL_CONN_EOF);
    if (rc > 0) *got=(size_t)rc;
    return status(ctx,rc);
}
int rs_tls_write(struct rs_tls *ctx, const unsigned char *b, size_t n, size_t *sent) {
    int rc; *sent=0;
    if (!owner(ctx) || ctx->failed || !ctx->handshaken || !n) return RS_TLS_ERROR;
    rc=mbedtls_ssl_write(&ctx->ssl,b,n); if (rc > 0) *sent=(size_t)rc;
    return status(ctx,rc);
}
int rs_tls_close(struct rs_tls *ctx) {
    if (!owner(ctx) || ctx->failed || !ctx->handshaken) return RS_TLS_ERROR;
    return status(ctx,mbedtls_ssl_close_notify(&ctx->ssl));
}
int rs_tls_error(struct rs_tls *ctx, struct rs_tls_error *error) {
    if (!owner(ctx)) return RS_TLS_ERROR;
    *error=ctx->error; return 0;
}
