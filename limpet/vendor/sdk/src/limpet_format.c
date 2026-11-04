/* The pure side of the SDK: reply decoding and sequence builders. No DOS
   calls, so the host tests exercise exactly the code that ships. */
#include <limpet/limpet.h>
#include <stdio.h>
#include <string.h>

void limpet_presentation_request(XtyControl *request)
{
    if (!request) return;
    memset(request, 0, sizeof *request);
    request->version = XTY_CONTROL_VERSION;
    request->operation = XTY_GET_PRESENTATION;
}

LimpetStatus limpet_presentation_decode(const XtyControl *reply,
                                        LimpetPresentation *out)
{
    LimpetPresentation p;
    uint32_t known;

    if (!out) return LIMPET_BAD_ARGUMENT;
    memset(out, 0, sizeof *out);
    if (!reply) return LIMPET_BAD_ARGUMENT;
    if (reply->version != XTY_CONTROL_VERSION || reply->operation != XTY_GET_PRESENTATION ||
        reply->reserved[0] != XTY_PACKET_MAGIC || reply->reserved[1])
        return LIMPET_INVALID_REPLY;
    known = reply->values[0];
    /* The protocol defines geometry as known only with both sides positive;
       a terminal claiming otherwise is broken, not merely unusual. */
    if ((known & XTY_PRESENT_KNOWN_GEOMETRY) && (!reply->values[4] || !reply->values[5]))
        return LIMPET_INVALID_REPLY;
    memset(&p, 0, sizeof p);
    p.known = known;
    if (known & XTY_PRESENT_KNOWN_COLORS) p.color_level = reply->values[1];
    if (known & XTY_PRESENT_KNOWN_STYLES) p.styles = reply->values[2];
    if (known & XTY_PRESENT_KNOWN_LINES) p.lines = reply->values[3];
    if (known & XTY_PRESENT_KNOWN_GEOMETRY) { p.rows = reply->values[4]; p.columns = reply->values[5]; }
    if (known & XTY_PRESENT_KNOWN_INTERPRETATION) p.interpretation = reply->values[6];
    if (known & XTY_PRESENT_KNOWN_INPUT) p.input = reply->values[7];
    if (known & XTY_PRESENT_KNOWN_FEATURES) p.features = reply->values[8];
    /* values[9..11] are reserved for later fields: ignored, not rejected,
       so a newer terminal keeps working with this client. */
    *out = p;
    return LIMPET_OK;
}

uint32_t limpet_color_level(const LimpetPresentation *p)
{
    if (!p || !(p->known & XTY_PRESENT_KNOWN_COLORS) || p->color_level > XTY_COLOR_RGB)
        return XTY_COLOR_NONE;
    return p->color_level;
}

int limpet_uses_pens(const LimpetPresentation *p)
{
    /* PENS without its known bit is no claim at all. */
    return p && (p->known & XTY_PRESENT_KNOWN_INTERPRETATION) &&
           (p->interpretation & XTY_COLOR_PENS);
}

int limpet_has_bold(const LimpetPresentation *p)
{
    return p && (p->known & XTY_PRESENT_KNOWN_STYLES) && (p->styles & XTY_STYLE_BOLD_RESET);
}

int limpet_can_rewrite_line(const LimpetPresentation *p)
{
    /* CR alone would leave the tail of a longer previous line on screen. */
    return p && (p->known & XTY_PRESENT_KNOWN_LINES) &&
           (p->lines & (XTY_LINE_CR | XTY_LINE_ERASE_EOL)) == (XTY_LINE_CR | XTY_LINE_ERASE_EOL);
}

uint32_t limpet_columns(const LimpetPresentation *p)
{
    return p && (p->known & XTY_PRESENT_KNOWN_GEOMETRY) ? p->columns : 0;
}

uint32_t limpet_rows(const LimpetPresentation *p)
{
    return p && (p->known & XTY_PRESENT_KNOWN_GEOMETRY) ? p->rows : 0;
}

/* snprintf into buf; an empty string and 0 when it does not fit, so a
   truncated escape sequence is never handed to a terminal. */
static size_t put(char *buf, size_t size, const char *format, unsigned a,
                  unsigned b, unsigned c, unsigned d)
{
    int n;
    if (!buf || !size) return 0;
    n = snprintf(buf, size, format, a, b, c, d);
    if (n <= 0 || (size_t)n >= size) { buf[0] = 0; return 0; }
    return (size_t)n;
}

static size_t nothing(char *buf, size_t size)
{
    if (buf && size) buf[0] = 0;
    return 0;
}

size_t limpet_sgr_ansi(char *buf, size_t size, const LimpetPresentation *p,
                       unsigned index, int background)
{
    unsigned base;
    if (index > 15 || limpet_color_level(p) < XTY_COLOR_ANSI16) return nothing(buf, size);
    /* With pens, 30..37 would pick a Workbench pen whose colour depends on
       the user's palette; 38;5;n keeps the ANSI meaning there. */
    if (limpet_uses_pens(p))
        return put(buf, size, "\033[%u;5;%um", background ? 48 : 38, index, 0, 0);
    base = index < 8 ? (background ? 40 : 30) : (background ? 100 : 90);
    return put(buf, size, "\033[%um", base + (index & 7), 0, 0, 0);
}

/* Nearest level of the xterm 6x6x6 cube: 0, 95, 135, 175, 215, 255. */
static unsigned cube_level(unsigned v)
{
    return v < 48 ? 0 : v < 115 ? 1 : (v - 35) / 40;
}

size_t limpet_sgr_rgb(char *buf, size_t size, const LimpetPresentation *p,
                      unsigned r, unsigned g, unsigned b, int background)
{
    uint32_t level = limpet_color_level(p);
    if (r > 255 || g > 255 || b > 255) return nothing(buf, size);
    if (level == XTY_COLOR_RGB)
        return put(buf, size, "\033[%u;2;%u;%u;%um", background ? 48 : 38, r, g, b);
    if (level == XTY_COLOR_INDEXED256)
        return put(buf, size, "\033[%u;5;%um", background ? 48 : 38,
                   16 + 36 * cube_level(r) + 6 * cube_level(g) + cube_level(b), 0, 0);
    return nothing(buf, size);
}

size_t limpet_osc8_open(char *buf, size_t size, const char *uri)
{
    size_t n, i;
    if (!buf || !size) return 0;
    if (!uri) return nothing(buf, size);
    n = strlen(uri);
    if (!n || n > LIMPET_OSC8_URI_MAX) return nothing(buf, size);
    /* ESC, BEL or any control would end the sequence early and let the
       rest of the URI run as terminal commands. Spaces are not valid in a
       URI either; percent-encode them. */
    for (i = 0; i < n; ++i)
        if ((unsigned char)uri[i] < 0x21 || (unsigned char)uri[i] > 0x7e) return nothing(buf, size);
    if (n + 7 >= size) return nothing(buf, size);
    memcpy(buf, "\033]8;;", 5);
    memcpy(buf + 5, uri, n);
    buf[5 + n] = '\033';
    buf[6 + n] = '\\';
    buf[7 + n] = 0;
    return n + 7;
}

int limpet_can_set_title(const LimpetPresentation *p)
{
    return p && (p->known & XTY_PRESENT_KNOWN_FEATURES) &&
           (p->features & XTY_FEATURE_TITLE);
}

/* Length of one valid UTF-8 scalar at s (n bytes left) that is not a
   control character, or 0. Overlong forms, surrogates and values past
   U+10FFFF are invalid. */
static size_t title_scalar(const unsigned char *s, size_t n)
{
    unsigned long c;
    size_t len, i;
    if (s[0] < 0x80) return s[0] >= 0x20 && s[0] != 0x7f;
    if (s[0] >= 0xc2 && s[0] <= 0xdf) { len = 2; c = s[0] & 0x1f; }
    else if (s[0] >= 0xe0 && s[0] <= 0xef) { len = 3; c = s[0] & 0x0f; }
    else if (s[0] >= 0xf0 && s[0] <= 0xf4) { len = 4; c = s[0] & 0x07; }
    else return 0;
    if (n < len) return 0;
    for (i = 1; i < len; ++i) {
        if ((s[i] & 0xc0) != 0x80) return 0;
        c = (c << 6) | (s[i] & 0x3f);
    }
    if ((len == 3 && c < 0x800) || (len == 4 && (c < 0x10000 || c > 0x10ffff)) ||
        (c >= 0xd800 && c <= 0xdfff) || (c >= 0x80 && c <= 0x9f))
        return 0;
    return len;
}

size_t limpet_osc_title(char *buf, size_t size, const LimpetPresentation *p,
                        const char *text)
{
    size_t n, i, step;
    if (!buf || !size) return 0;
    if (!text || !limpet_can_set_title(p)) return nothing(buf, size);
    n = strlen(text);
    if (n > LIMPET_TITLE_MAX) return nothing(buf, size);
    for (i = 0; i < n; i += step)
        if (!(step = title_scalar((const unsigned char *)text + i, n - i)))
            return nothing(buf, size);
    if (n + 6 >= size) return nothing(buf, size);
    memcpy(buf, "\033]2;", 4);
    memcpy(buf + 4, text, n);
    buf[4 + n] = '\033';
    buf[5 + n] = '\\';
    buf[6 + n] = 0;
    return n + 6;
}

static const char base64_table[] =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

int limpet_kitty_transmit(const LimpetImage *image, const unsigned char *pixels,
                          LimpetWriteFn write, void *context)
{
    /* One escape sequence: 64 bytes of keys, the chunk, framing. */
    char sequence[LIMPET_KITTY_CHUNK + 96];
    size_t bytes, at = 0, channels;
    int first = 1, rc;

    if (!image || !pixels || !write || !image->id ||
        image->width < 1 || image->width > LIMPET_KITTY_MAX_SIDE ||
        image->height < 1 || image->height > LIMPET_KITTY_MAX_SIDE ||
        (image->format != 24 && image->format != 32) || image->quiet > 2)
        return -1;
    channels = image->format / 8;
    bytes = (size_t)image->width * image->height * channels;
    if (bytes > LIMPET_KITTY_MAX_BYTES) return -1;
    while (at < bytes) {
        /* 3072 raw bytes are exactly 4096 base64 characters, so only the
           last chunk can carry padding, as the protocol requires. */
        size_t raw = bytes - at > LIMPET_KITTY_CHUNK / 4 * 3 ? LIMPET_KITTY_CHUNK / 4 * 3 : bytes - at;
        int more = at + raw < bytes;
        size_t n, i;
        if (first)
            n = (size_t)snprintf(sequence, sizeof sequence,
                                 "\033_Ga=T,f=%u,t=d,s=%u,v=%u,i=%lu,C=%d,q=%u,m=%d;",
                                 image->format, image->width, image->height,
                                 (unsigned long)image->id, image->keep_cursor ? 1 : 0,
                                 image->quiet, more);
        else
            n = (size_t)snprintf(sequence, sizeof sequence, "\033_Gm=%d;", more);
        for (i = 0; i < raw; i += 3) {
            const unsigned char *s = pixels + at + i;
            unsigned v = (unsigned)s[0] << 16 | (i + 1 < raw ? (unsigned)s[1] << 8 : 0) |
                         (i + 2 < raw ? s[2] : 0);
            sequence[n++] = base64_table[v >> 18 & 63];
            sequence[n++] = base64_table[v >> 12 & 63];
            sequence[n++] = i + 1 < raw ? base64_table[v >> 6 & 63] : '=';
            sequence[n++] = i + 2 < raw ? base64_table[v & 63] : '=';
        }
        sequence[n++] = '\033';
        sequence[n++] = '\\';
        /* A whole sequence per call: another writer on the same console
           cannot land in the middle of an escape sequence of ours. */
        rc = write(context, sequence, n);
        if (rc) {
            /* Limpet keeps an unfinished transmission, and its buffer,
               until another command arrives. A final empty chunk ends it
               with a wrong byte count, so the terminal drops it now. Best
               effort: the writer has just failed. */
            if (!first) write(context, "\033_Gm=0;\033\\", 9);
            return rc;
        }
        at += raw;
        first = 0;
    }
    return 0;
}

int limpet_kitty_delete(uint32_t id, int free_data, LimpetWriteFn write, void *context)
{
    char sequence[64];
    int n;
    if (!write) return -1;
    if (id)
        n = snprintf(sequence, sizeof sequence, "\033_Ga=d,d=%c,i=%lu,q=2\033\\",
                     free_data ? 'I' : 'i', (unsigned long)id);
    else
        n = snprintf(sequence, sizeof sequence, "\033_Ga=d,d=%c,q=2\033\\", free_data ? 'A' : 'a');
    return write(context, sequence, (size_t)n);
}

size_t limpet_kitty_query(char *buf, size_t size, uint32_t id)
{
    int n;
    if (!buf || !size) return 0;
    if (!id) return nothing(buf, size);
    /* One black RGB pixel: base64 of three zero bytes is AAAA. */
    n = snprintf(buf, size, "\033_Gi=%lu,s=1,v=1,a=q,t=d,f=24;AAAA\033\\", (unsigned long)id);
    if (n <= 0 || (size_t)n >= size) return nothing(buf, size);
    return (size_t)n;
}

int limpet_kitty_parse_reply(const char *bytes, size_t length, uint32_t id,
                             char *message, size_t message_size)
{
    char expected[32];
    size_t start, end, prefix, text;
    int n;

    if (message && message_size) message[0] = 0;
    if (!bytes || !id) return -1;
    /* Find ESC _ G; anything before it (a stray key press) is skipped. */
    for (start = 0; start + 2 < length; ++start)
        if (bytes[start] == '\033' && bytes[start + 1] == '_' && bytes[start + 2] == 'G') break;
    if (start + 2 >= length) return -1;
    for (end = start + 3; end + 1 < length; ++end)
        if (bytes[end] == '\033' && bytes[end + 1] == '\\') break;
    if (end + 1 >= length) return -1;
    n = snprintf(expected, sizeof expected, "i=%lu;", (unsigned long)id);
    prefix = (size_t)n;
    if (end - (start + 3) < prefix || memcmp(bytes + start + 3, expected, prefix)) return -1;
    text = end - (start + 3 + prefix);
    if (text == 2 && !memcmp(bytes + start + 3 + prefix, "OK", 2)) return 1;
    if (message && message_size) {
        size_t copy = text < message_size - 1 ? text : message_size - 1;
        memcpy(message, bytes + start + 3 + prefix, copy);
        message[copy] = 0;
    }
    return 0;
}
