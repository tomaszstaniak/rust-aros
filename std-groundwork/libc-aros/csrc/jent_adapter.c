/* Reuse arospkg's adapter unchanged, under private names so an application
 * can link Mbed TLS and its own adapter without symbol collisions. */
#define mbedtls_hardware_poll rs_jent_poll
#define aros_jent_status rs_jent_status
#define aros_jent_shutdown rs_jent_shutdown
#include "vendor/arospkg-entropy/entropy_jent.c"
#include <aros/symbolsets.h>
ADD2EXIT(rs_jent_shutdown, 10);
