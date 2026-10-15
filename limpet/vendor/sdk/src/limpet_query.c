/* The DOS side of the SDK: the only file that sends packets. */
#include <limpet/limpet.h>
#include <dos/dosextens.h>
#include <proto/dos.h>
#include <string.h>

LONG limpet_control(BPTR handle, XtyControl *control)
{
    struct FileHandle *fh;
    XtyControl copy;
    SIPTR result, error;

    if (!handle || !control) { SetIoErr(ERROR_BAD_NUMBER); return DOSFALSE; }
    fh = BADDR(handle);
    /* NIL: and closed handles have no handler port to ask. */
    if (!fh->fh_Type) { SetIoErr(ERROR_INVALID_LOCK); return DOSFALSE; }
    /* Work on a copy: the caller's block must stay untouched on failure,
       and a handler that does not know the action may scribble on it. */
    copy = *control;
    copy.reserved[0] = 0;
    result = DoPkt(fh->fh_Type, XTY_ACTION_CONTROL, fh->fh_Arg1, (SIPTR)&copy,
                   sizeof copy, XTY_PACKET_MAGIC, 0);
    error = IoErr();
    /* Validate in packet width before narrowing: on x86_64 0xffffffff is
       not DOSTRUE, and a success that carries an error code is malformed. */
    if (error < 0 || error > INT32_MAX ||
        (result != DOSFALSE && result != DOSTRUE) || (result == DOSTRUE && error)) {
        SetIoErr(ERROR_BAD_NUMBER);
        return DOSFALSE;
    }
    if (result != DOSTRUE) {
        /* Some handlers refuse an unknown action without an error code. */
        if (!error) SetIoErr(ERROR_ACTION_NOT_KNOWN);
        return DOSFALSE;
    }
    /* reserved[0] is zero on the way out and the magic on the way back:
       a handler that merely returned the block unchanged did not answer. */
    if (copy.version != control->version || copy.operation != control->operation ||
        copy.reserved[0] != XTY_PACKET_MAGIC || copy.reserved[1]) {
        SetIoErr(ERROR_ACTION_NOT_KNOWN);
        return DOSFALSE;
    }
    *control = copy;
    SetIoErr(0);
    return DOSTRUE;
}

LimpetStatus limpet_query_presentation(BPTR output, LimpetPresentation *out,
                                       LONG *ioerr)
{
    XtyControl request;
    LONG error;
    LimpetStatus status;

    if (ioerr) *ioerr = 0;
    if (!out) return LIMPET_BAD_ARGUMENT;
    memset(out, 0, sizeof *out);
    if (!output) { if (ioerr) *ioerr = ERROR_INVALID_LOCK; return LIMPET_BAD_ARGUMENT; }
    /* Files and pipes never get the packet: a filesystem has no reason to
       know the action, and the answer would not matter anyway. */
    if (!IsInteractive(output)) return LIMPET_NOT_INTERACTIVE;
    limpet_presentation_request(&request);
    if (limpet_control(output, &request) != DOSTRUE) {
        error = IoErr();
        if (ioerr) *ioerr = error;
        return error == ERROR_ACTION_NOT_KNOWN ? LIMPET_UNSUPPORTED :
               error == ERROR_BAD_NUMBER ? LIMPET_INVALID_REPLY : LIMPET_FAILED;
    }
    status = limpet_presentation_decode(&request, out);
    if (status != LIMPET_OK && ioerr) *ioerr = ERROR_BAD_NUMBER;
    return status;
}

int limpet_write_dos(void *context, const char *bytes, size_t length)
{
    BPTR handle = (BPTR)context;
    /* Write() may accept fewer bytes than asked; resending the whole
       buffer would repeat the prefix that did arrive. */
    while (length) {
        LONG chunk = length > 0x7fffffff ? 0x7fffffff : (LONG)length;
        LONG n = Write(handle, (APTR)bytes, chunk);
        if (n <= 0) return 1;
        bytes += n;
        length -= (size_t)n;
    }
    return 0;
}
