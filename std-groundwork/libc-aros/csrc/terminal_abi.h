/* Build-time check of src/terminal.rs against the selected SDK's headers.
 * Included by compat.c, so an SDK with another terminal ABI fails the build
 * instead of corrupting a caller's termios. */
#include <termios.h>
#include <sys/ioctl.h>
#include <stddef.h>
#define RS_TERMINAL_CHECK(x) _Static_assert((x), #x)
RS_TERMINAL_CHECK(sizeof(struct termios) == 112 && _Alignof(struct termios) == 4);
RS_TERMINAL_CHECK(offsetof(struct termios, c_iflag) == 0);
RS_TERMINAL_CHECK(offsetof(struct termios, c_oflag) == 4);
RS_TERMINAL_CHECK(offsetof(struct termios, c_cflag) == 8);
RS_TERMINAL_CHECK(offsetof(struct termios, c_lflag) == 12);
RS_TERMINAL_CHECK(offsetof(struct termios, c_cc) == 16);
RS_TERMINAL_CHECK(offsetof(struct termios, internal) == 48);
RS_TERMINAL_CHECK(sizeof(struct winsize) == 8 && _Alignof(struct winsize) == 2);
RS_TERMINAL_CHECK(offsetof(struct winsize, ws_row) == 0);
RS_TERMINAL_CHECK(offsetof(struct winsize, ws_col) == 2);
RS_TERMINAL_CHECK(offsetof(struct winsize, ws_xpixel) == 4);
RS_TERMINAL_CHECK(offsetof(struct winsize, ws_ypixel) == 6);
RS_TERMINAL_CHECK(TIOCGWINSZ == 0x40087468);
RS_TERMINAL_CHECK(TCSANOW == 0 && TCSADRAIN == 1 && TCSAFLUSH == 2);
RS_TERMINAL_CHECK(NCCS == 32 && VEOF == 0 && VEOL == 1 && VERASE == 2 && VINTR == 3);
RS_TERMINAL_CHECK(VKILL == 4 && VMIN == 5 && VQUIT == 6 && VSTART == 7);
RS_TERMINAL_CHECK(VSTOP == 8 && VSUSP == 9 && VTIME == 10);
RS_TERMINAL_CHECK(BRKINT == 01 && ICRNL == 02 && IGNBRK == 04 && IGNCR == 010);
RS_TERMINAL_CHECK(IGNPAR == 020 && INLCR == 040 && INPCK == 0100 && ISTRIP == 0200);
RS_TERMINAL_CHECK(IXANY == 0400 && IXOFF == 01000 && IXON == 02000 && PARMRK == 04000);
RS_TERMINAL_CHECK(OPOST == 01 && ONLCR == 02);
RS_TERMINAL_CHECK(CSIZE == 03 && CS8 == 03 && PARENB == 020);
RS_TERMINAL_CHECK(ECHO == 01 && ECHOE == 02 && ECHOK == 04 && ECHONL == 010);
RS_TERMINAL_CHECK(ICANON == 020 && IEXTEN == 0100 && ISIG == 0200);
#undef RS_TERMINAL_CHECK
