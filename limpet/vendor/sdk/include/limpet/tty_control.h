#ifndef XTY_CONTROL_H
#define XTY_CONTROL_H
#include <stdint.h>
#define XTY_ACTION_CONTROL 0x58545901L
#define XTY_ACTION_CANCEL 0x58545902L
#define XTY_ACTION_TRY_READ 0x58545903L
#define XTY_ACTION_TRY_WRITE 0x58545904L
#define XTY_CAP_TRY_IO 32768u
/* Immediate I/O: Arg1 export, Arg2 buffer, Arg3 length [0, INT32_MAX],
   Arg4 magic, Arg5 zero. Count >=0 / error zero succeeds (read zero = EOF
   unless length zero). -1 / ERROR_OBJECT_IN_USE means retry, NOT EOF.
   No request retained waiting for data/capacity; scheduling/IPC still waits. */
#define XTY_CAP_CANCEL 8192u
/* Native DOS cancellation, not a pointer-bearing XtyControl extension:
   Arg1 live export, Arg2 original packet identity, Arg3/5 zero, Arg4 magic.
   Only a pending request on this export and the same reply port can match.
   Caller must retain both packets and collect both replies. */
#define XTY_PACKET_MAGIC 0x58545931L
#define XTY_CONTROL_VERSION 1u
#define XTY_CONTROL_BYTES 64u
enum { XTY_CAPS=1, XTY_GET_CONFIG, XTY_SET_CONFIG, XTY_GET_GEOMETRY,
       XTY_SET_GEOMETRY, XTY_GET_READ_STATE, XTY_FLUSH, XTY_SET_CONFIG_DISCARD_INPUT,
       XTY_DRAIN, XTY_DRAIN_CONFIG, XTY_DRAIN_CONFIG_DISCARD_INPUT, XTY_OUTPUT_FLOW,
       XTY_GET_PRESENTATION };
/* Operation 13: renderer-owned snapshot on the actual output FileHandle.
   Request: reserved and values all zero. Reply values: [0] known mask,
   [1] color level, [2] styles, [3] line controls, [4] rows, [5] columns,
   [6] color interpretation (known bit 4), [7] input features (known bit
   32, Limpet 0.2.0 and later; 0.1.0 leaves both clear); [8..11] zero. Clear known bits
   mean unknown, not unsupported. INPUT_BRACKETED_PASTE: in raw mode the
   client may set DEC mode 2004 (CSI ? 2004 h) and then receives a paste
   framed by CSI 200 ~ and CSI 201 ~, without Limpet's multi-line
   confirmation; the text itself never contains ESC or C1.
   INPUT_RESIZE_REPORT: in raw mode the client may set DEC mode 2048 and
   then receives CSI 48 ; rows ; columns ; height px ; width px t as input
   at once and after every size change.
   INPUT_WHEEL_REPORT: in raw mode the client may set Limpet's private mode
   2100 and then receives each wheel step as CSI < 64|65 ; col ; row M
   (modifier bits 8 Alt, 16 Ctrl) while buttons and drags stay with the
   terminal; mouse tracking 1000/1002, when also on, takes precedence.
   No report or framed paste ever goes inside a framed paste. A native
   console turns all of these modes (and 1000/1002/1006) off when it
   returns to cooked mode; set them again after each SetMode(raw).
   All promises refer to 7-bit ESC [ (0x1b,0x5b) sequences.
   ANSI16: SGR 30..37,90..97,39 (foreground), 40..47,100..107,49
   (background). INDEXED256 adds 38;5;n and 48;5;n; RGB adds
   38;2;r;g;b and 48;2;r;g;b. Semicolon forms only.
   COLOR_PENS in [6], valid only with KNOWN_INTERPRETATION in [0]: SGR
   30..37 and 40..47 select Amiga screen pens 0..7, as in the AROS
   console, not ANSI colours. With it, 38;5;n and 48;5;n for n 0..15 are
   guaranteed from level ANSI16 up and keep their ANSI palette meaning.
   SGR 39 restores the theme's default foreground and drops a foreground
   pen; it does not restore an earlier pen and leaves background and bold
   alone (49 likewise for the background). A Shell prompt sets its own
   pens with its own sequences. PENS without the known bit is no claim;
   unknown interpretation leaves the client's ANSI behaviour unchanged.
   BOLD_RESET: SGR 1 and 0. CR: byte 0x0d. ERASE_EOL: CSI K / CSI 0 K.
   FEATURES in [8], valid only with KNOWN_FEATURES in [0] (Limpet 0.3.0;
   0.2.x and earlier leave both clear, so a reply alone never means a
   feature below exists). FEATURE_TITLE: OSC 0 and OSC 2 set the window
   title: ESC ] 0|2 ; text BEL or ESC ] 0|2 ; text ESC \, text 1..255
   bytes of UTF-8 without C0, DEL or C1 controls (anything else drops the
   whole sequence), empty text drops the program's title. The title lasts
   while the command that was in the foreground when it arrived runs;
   characters outside Latin-1 show as '?'. CON: prints OSC as text.
   Generic PTYs do not imply presentation support. This query neither
   reads keyboard input nor changes terminal modes. No response deadline. */
#define XTY_PRESENT_KNOWN_COLORS 1u
#define XTY_PRESENT_KNOWN_STYLES 2u
#define XTY_PRESENT_KNOWN_LINES 4u
#define XTY_PRESENT_KNOWN_GEOMETRY 8u
#define XTY_PRESENT_KNOWN_INTERPRETATION 16u
#define XTY_PRESENT_KNOWN_INPUT 32u
#define XTY_PRESENT_KNOWN_FEATURES 64u
#define XTY_INPUT_BRACKETED_PASTE 1u
#define XTY_INPUT_RESIZE_REPORT 2u
#define XTY_INPUT_WHEEL_REPORT 4u
#define XTY_FEATURE_TITLE 1u
#define XTY_COLOR_PENS 1u
#define XTY_COLOR_NONE 0u
#define XTY_COLOR_ANSI16 1u
#define XTY_COLOR_INDEXED256 2u
#define XTY_COLOR_RGB 3u
#define XTY_STYLE_BOLD_RESET 1u
#define XTY_LINE_CR 1u
#define XTY_LINE_ERASE_EOL 2u
#define XTY_CAP_OUTPUT_FLOW 4096u
#define XTY_OUTPUT_STOP 1u
#define XTY_OUTPUT_START 2u
/* Operation 12: slave-only, values[0] STOP/START, values[1..11] zero.
   Success returns zero values. Not ordered behind pending writes/drains. */
#define XTY_CAP_GET_CONFIG 1u
#define XTY_CAP_SET_CONFIG 2u
#define XTY_CAP_GET_GEOMETRY 4u
#define XTY_CAP_SET_GEOMETRY 8u
#define XTY_CAP_GET_READ_STATE 16u
#define XTY_CAP_FLUSH 32u
#define XTY_CAP_SET_CONFIG_DISCARD_INPUT 64u
#define XTY_CAP_DRAIN 128u
#define XTY_CAP_DRAIN_CONFIG 256u
#define XTY_CAP_DRAIN_CONFIG_DISCARD_INPUT 512u
/* Operations 9..11 are asynchronous inside the owner, synchronous for DoPkt.
   DRAIN values must be zero. Combined operations use SET_CONFIG's payload.
   Owner retains the block until reply; failure leaves it unchanged. */
/* Operation 8 uses SET_CONFIG's payload/reply, atomically discarding accepted
   input only after validation. No output drain: NOT POSIX TCSAFLUSH. */
/* Slave-only, values[0] selects queues; values[1..11] must be zero.
   Discards accepted bytes, not pending writes or completed read replies. */
#define XTY_FLUSH_INPUT 1u
#define XTY_FLUSH_OUTPUT 2u
/* Non-consuming snapshot, not a reservation, subscription or byte count.
   EOF includes a pending canonical VEOF as well as drained peer closure. */
#define XTY_READ_WAIT 0u
#define XTY_READ_DATA 1u
#define XTY_READ_EOF 2u
#define XTY_ICRNL 1u
#define XTY_IGNCR 2u
#define XTY_INLCR 4u
#define XTY_OPOST 1u
#define XTY_ONLCR 2u
#define XTY_CANON 1u
#define XTY_ECHO 2u
#define XTY_ECHOE 4u
#define XTY_ECHOK 8u
#define XTY_ECHONL 16u
#define XTY_NOEOF 32u
#define XTY_ISIG 64u
#define XTY_NOFLSH 128u
#define XTY_LOCAL_MASK 255u
#define XTY_CAP_VINTR 16384u
/* Config values[9]: VINTR, zero disabled, otherwise byte+1 (1..256).
   Negotiate CAP_VINTR for ISIG/NOFLSH or nonzero VINTR. Single-target Ctrl-C
   subset only; VQUIT/VSUSP remain disabled. values[10..11] remain zero. */
#define XTY_CAP_CANONICAL_EDIT 1024u
#define XTY_CAP_READ_TIMING 2048u
/* Config values[7]: 0 = legacy VMIN=1, else VMIN+1 (1..256).
   values[8]: VTIME in deciseconds (0..255). Service/worker capability only. */
/* Config values[4..6]: erase/kill/EOL, zero disabled, otherwise byte+1.
   NOEOF disables values[3]. Negotiate the
   CANONICAL_EDIT capability before sending these extensions. */
/* Experimental, native-endian, synchronous same-machine protocol, not termios.
   Caller owns the entire block until the DOS reply. No embedded pointers. */
typedef struct {
    uint32_t version,operation,reserved[2],values[12];
} XtyControl;
typedef char XtyControlSizeCheck[sizeof(XtyControl)==XTY_CONTROL_BYTES?1:-1];
#endif
