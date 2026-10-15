/* Limpet SDK: ask the terminal what it renders, then write terminal
   sequences that match the answer.

   Two layers, kept apart on purpose:

   - tty_control.h is the wire format. It is the header the terminal itself
     is built from; XtyControl, the operation numbers and the bit values are
     the protocol and do not change with SDK releases.
   - This header is convenience API on top of it. Everything here can be
     written by hand from tty_control.h; the functions exist so that every
     client validates replies and treats unknown fields the same way.

   Thread and handle rules: a call uses the DOS FileHandle it is given and
   nothing else. The caller keeps that handle open for the whole call and
   does not use it from another task at the same time. No function keeps a
   pointer after it returns, and none allocates memory.

   Reference: doc/sdk/api.md. */
#ifndef LIMPET_LIMPET_H
#define LIMPET_LIMPET_H

#include <stddef.h>
#include <stdint.h>
#include <dos/dos.h>
#include "tty_control.h"

#define LIMPET_SDK_VERSION_MAJOR 0
#define LIMPET_SDK_VERSION_MINOR 3
#define LIMPET_SDK_VERSION_PATCH 0
#define LIMPET_SDK_VERSION_STRING "0.3.0"

/* Binary guard. SDK 0.2 appended `input` and SDK 0.3 `features` to
   LimpetPresentation, so a function that fills the struct writes more than
   an older client reserved. These functions therefore have new link names
   in each such release: source code is unchanged, but an object compiled
   with an older header no longer links against this library and must be
   rebuilt, instead of having its stack overwritten. */
#define limpet_query_presentation limpet_query_presentation_v3
#define limpet_presentation_decode limpet_presentation_decode_v3

#ifdef __cplusplus
extern "C" {
#endif

/* ---- Presentation query (operation 13) ---- */

typedef enum {
    LIMPET_OK = 0,
    /* The handle is not interactive (a file, a pipe, NIL:). No packet was
       sent. Write plain text without escape sequences. */
    LIMPET_NOT_INTERACTIVE = 1,
    /* The console does not answer the query: CON:, a console that is not
       Limpet, or a Limpet session that is closing. IoErr() was
       ERROR_ACTION_NOT_KNOWN. Nothing is known about the console. */
    LIMPET_UNSUPPORTED = 2,
    /* The handler answered, but the reply broke the protocol. Treat it as
       LIMPET_UNSUPPORTED; it is reported separately so that it can be
       logged. */
    LIMPET_INVALID_REPLY = 3,
    /* Any other DOS failure; the error code is in *ioerr. */
    LIMPET_FAILED = 4,
    /* An argument was NULL. */
    LIMPET_BAD_ARGUMENT = 5
} LimpetStatus;

/* A decoded reply. Each field is meaningful only when its known bit is set;
   the decoder zeroes a field whose bit is clear, so a client that forgets
   to test a bit sees "none", never stale data.

   A clear known bit means UNKNOWN, not unsupported. A set known bit with a
   zero value means confirmed absent. */
typedef struct {
    uint32_t known;          /* XTY_PRESENT_KNOWN_* bits, including bits this
                                SDK does not know yet */
    uint32_t color_level;    /* XTY_COLOR_NONE .. XTY_COLOR_RGB, or a newer
                                value (see limpet_color_level) */
    uint32_t styles;         /* XTY_STYLE_* bits */
    uint32_t lines;          /* XTY_LINE_* bits */
    uint32_t rows, columns;  /* text grid when the reply was made */
    uint32_t interpretation; /* XTY_COLOR_PENS and future bits */
    uint32_t input;          /* XTY_INPUT_* bits: modes a raw-mode client may
                                turn on (XTY_PRESENT_KNOWN_INPUT) */
    uint32_t features;       /* XTY_FEATURE_* bits (XTY_PRESENT_KNOWN_FEATURES,
                                Limpet 0.3.0 and later) */
} LimpetPresentation;

/* Zero the block and set version and operation. Every other field must be
   zero on the wire; this is the only correct way to start a request. */
void limpet_presentation_request(XtyControl *request);

/* Check and decode a reply block that has already been received (for
   example by limpet_control). Returns LIMPET_OK or LIMPET_INVALID_REPLY.
   Reserved values[8..11] and unknown bits are accepted and ignored, so a
   newer terminal does not break an older client. On failure *out is
   zeroed. Pure function; usable on any host. */
LimpetStatus limpet_presentation_decode(const XtyControl *reply,
                                        LimpetPresentation *out);

/* Ask the console behind `output` what it renders. Pass the handle you are
   going to write to, normally Output(); never Input(). With `prog >file`
   Input() is still the terminal, but the text goes to the file.

   Flush buffered stdio output first (fflush(stdout)) if the answer will
   decide how to write text that follows it: the query travels as a DOS
   packet and is not ordered behind data still in a C library buffer.

   BLOCKS until the console's handler replies. Limpet and CON: reply at
   once; there is no timeout, because DOS packets have none. Does not read
   input and does not change terminal modes.

   *out is zeroed unless LIMPET_OK. ioerr may be NULL; when given it
   receives the DOS error code (0 on success). */
LimpetStatus limpet_query_presentation(BPTR output, LimpetPresentation *out,
                                       LONG *ioerr);

/* Send one XtyControl request on a live handle and validate the reply
   envelope (version, operation, response marker). DOSTRUE on success with
   the reply copied into *control; DOSFALSE with IoErr() set otherwise, and
   *control is left unchanged. A handler that answers DOSFALSE with no
   error code is reported as ERROR_ACTION_NOT_KNOWN.

   This is the raw transport under limpet_query_presentation. Limpet 0.1.0
   serves only XTY_GET_PRESENTATION on a Shell session; other operations
   get ERROR_ACTION_NOT_KNOWN there. BLOCKS like the query above. */
LONG limpet_control(BPTR handle, XtyControl *control);

/* ---- Decisions from a decoded reply (pure, never fail) ---- */

/* The colour level, or XTY_COLOR_NONE when colours are unknown or the
   value is newer than this SDK. A newer level is not assumed to include
   the older ones: this SDK cannot know. */
uint32_t limpet_color_level(const LimpetPresentation *p);

/* 1 when the terminal said that SGR 30..37 / 40..47 select Amiga screen
   pens 0..7 instead of ANSI colours. Then use limpet_sgr_ansi, which
   writes 38;5;n. */
int limpet_uses_pens(const LimpetPresentation *p);

/* 1 when SGR 1 (bold) and SGR 0 (reset) are confirmed. */
int limpet_has_bold(const LimpetPresentation *p);

/* 1 when CR and CSI K are both confirmed: a line can be rewritten in
   place (progress bars). */
int limpet_can_rewrite_line(const LimpetPresentation *p);

/* Columns, or 0 when unknown. A snapshot: it does not follow a resize. */
uint32_t limpet_columns(const LimpetPresentation *p);
uint32_t limpet_rows(const LimpetPresentation *p);

/* 1 when the terminal confirmed that OSC 0/2 set its window title
   (XTY_FEATURE_TITLE). Limpet 0.2.x answers the query but does not
   confirm it; never send a title then: CON: would print it as text. */
int limpet_can_set_title(const LimpetPresentation *p);

/* ---- Sequence builders (pure) ----

   Each writes a NUL-terminated sequence into buf and returns its length
   without the NUL. A return of 0 means "write nothing": the feature is not
   confirmed, an argument is out of range, or buf is too small. buf is
   left as an empty string in that case (when size > 0). */

#define LIMPET_SGR_RESET  "\033[0m"   /* all attributes */
#define LIMPET_SGR_BOLD   "\033[1m"
#define LIMPET_SGR_FG_DEFAULT "\033[39m" /* theme foreground, keeps bold */
#define LIMPET_SGR_BG_DEFAULT "\033[49m"
#define LIMPET_ERASE_EOL  "\033[K"
#define LIMPET_SEQUENCE_MAX 32        /* enough for any SGR builder */

/* One of the 16 ANSI colours (0..7 normal, 8..15 bright), keeping its
   palette meaning: 38;5;n / 48;5;n when the terminal uses pens, otherwise
   30..37 / 90..97 (40..47 / 100..107). Needs colour level ANSI16 or more. */
size_t limpet_sgr_ansi(char *buf, size_t size, const LimpetPresentation *p,
                       unsigned index, int background);

/* A 24-bit colour: 38;2;r;g;b at level RGB, the nearest 6x6x6 cube entry
   (38;5;16..231) at level INDEXED256, nothing below that. Pick your own
   ANSI fallback for level ANSI16; only you know which of the 16 is close
   enough for your purpose. */
size_t limpet_sgr_rgb(char *buf, size_t size, const LimpetPresentation *p,
                      unsigned r, unsigned g, unsigned b, int background);

/* OSC 8 start: ESC ] 8 ; ; uri ESC \. The URI must be 1..2047 bytes of
   printable ASCII (0x21..0x7e); anything else returns 0, so text from
   outside can never smuggle in an escape sequence. Limpet opens only
   http:// and https:// targets; other schemes are shown but inert. Close
   the link with LIMPET_OSC8_CLOSE. */
#define LIMPET_OSC8_URI_MAX 2047
#define LIMPET_OSC8_CLOSE "\033]8;;\033\\"
size_t limpet_osc8_open(char *buf, size_t size, const char *uri);

/* Window title: ESC ] 2 ; text ESC \. text is 1..LIMPET_TITLE_MAX bytes
   of valid UTF-8 without control characters (C0, DEL, C1); anything else
   returns 0, as does a terminal that did not confirm XTY_FEATURE_TITLE.
   text "" builds the reset, LIMPET_TITLE_RESET: the terminal shows the
   running command's name again. Limpet keeps a title while the command
   that sent it runs and drops it when that command ends, so a program
   need not reset it on exit. Send a title when your state changes, not
   with every write: the terminal applies at most about four per second. */
#define LIMPET_TITLE_MAX 255
#define LIMPET_TITLE_RESET "\033]2;\033\\"
size_t limpet_osc_title(char *buf, size_t size, const LimpetPresentation *p,
                        const char *text);

/* ---- Kitty graphics, the subset Limpet implements ----

   Detection: a successful presentation query identifies Limpet; the Kitty
   query (limpet_kitty_query) is the protocol's own test and needs raw mode
   to read its reply. Never send image data to a console that did not
   answer either: CON: prints the payload as text. */

#define LIMPET_KITTY_MAX_SIDE 1024u          /* pixels, width and height */
#define LIMPET_KITTY_MAX_BYTES (4u << 20)    /* decoded pixels per image */
#define LIMPET_KITTY_MAX_IMAGES 16u          /* per terminal */
#define LIMPET_KITTY_CHUNK 4096u             /* base64 bytes per sequence */

/* Output callback: write all `length` bytes or fail. Return 0 on success,
   nonzero to stop. limpet_write_dos below is a ready one for a BPTR. */
typedef int (*LimpetWriteFn)(void *context, const char *bytes, size_t length);

/* context is a BPTR cast to (void *): limpet_write_dos((void *)Output(),..).
   Retries short writes; fails on a zero or negative Write(). */
int limpet_write_dos(void *context, const char *bytes, size_t length);

typedef struct {
    uint32_t id;          /* 1..4294967295; reusing an id replaces it */
    unsigned width;       /* 1..LIMPET_KITTY_MAX_SIDE */
    unsigned height;      /* 1..LIMPET_KITTY_MAX_SIDE */
    unsigned format;      /* 24 (RGB) or 32 (RGBA, blended over text) */
    int keep_cursor;      /* 1: C=1, cursor stays; 0: moves past the image */
    unsigned quiet;       /* 0 reply always, 1 errors only, 2 never */
} LimpetImage;

/* Transmit and display (a=T) at the cursor, base64 in chunks of
   LIMPET_KITTY_CHUNK, one whole escape sequence per write call.
   `pixels` holds width*height*format/8 bytes, rows top to bottom.
   Returns 0 on success, -1 on invalid arguments (nothing written), or the
   callback's nonzero result. After a failure past the first chunk an
   empty final chunk is attempted, so the terminal drops the partial image. */
int limpet_kitty_transmit(const LimpetImage *image, const unsigned char *pixels,
                          LimpetWriteFn write, void *context);

/* Delete the image's placement; free_data also frees its pixels.
   id 0 deletes every placement. Returns 0, -1 or the callback's result. */
int limpet_kitty_delete(uint32_t id, int free_data,
                        LimpetWriteFn write, void *context);

/* The support query: a 1x1 RGB a=q with the given id. The terminal answers
   ESC _ G i=<id>;OK ESC \ on Input(), in raw mode only. */
size_t limpet_kitty_query(char *buf, size_t size, uint32_t id);

/* Parse one reply (ESC _ G ... ESC \). Returns 1 for OK, 0 for an error
   reply (its text, e.g. "ENOTSUPPORTED:...", is copied to message), -1 if
   bytes do not form a reply for `id`. message may be NULL. */
int limpet_kitty_parse_reply(const char *bytes, size_t length, uint32_t id,
                             char *message, size_t message_size);

#ifdef __cplusplus
}
#endif
#endif
