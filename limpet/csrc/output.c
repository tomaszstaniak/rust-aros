/* Keep DOS typedefs and SDK layout on the C side of the Rust boundary. */
#include <stdint.h>
#include <stddef.h>
#include <limpet/limpet.h>
#include <proto/dos.h>
_Static_assert(sizeof(LimpetPresentation) == 9 * sizeof(uint32_t), "presentation layout");
_Static_assert(offsetof(LimpetPresentation, features) == 8 * sizeof(uint32_t), "feature offset");
int rs_limpet_current(uintptr_t *handle, LimpetPresentation *presentation, int64_t *error)
{
    LONG code = 0;
    BPTR output = Output();
    *handle = (uintptr_t)output;
    LimpetStatus status = limpet_query_presentation(output, presentation, &code);
    *error = (int64_t)code;
    return (int)status;
}
int rs_limpet_write(uintptr_t handle, const unsigned char *bytes, size_t len)
{
    return limpet_write_dos((void *)handle, (const char *)bytes, len);
}
