/* Host stand-in for <dos/dos.h>: only what the SDK uses. Test-only; the
   types are wide like on x86_64 AROS, so narrowing bugs show up here. */
#ifndef LIMPET_HOST_DOS_DOS_H
#define LIMPET_HOST_DOS_DOS_H
#include <stdint.h>
typedef int32_t LONG;
typedef intptr_t SIPTR;
typedef intptr_t BPTR;
typedef void *APTR;
#define DOSTRUE (-1)
#define DOSFALSE 0
#define BADDR(x) ((void *)(x))
#define ERROR_BAD_NUMBER 115
#define ERROR_INVALID_LOCK 211
#define ERROR_ACTION_NOT_KNOWN 209
#define ERROR_OBJECT_IN_USE 202
#endif
