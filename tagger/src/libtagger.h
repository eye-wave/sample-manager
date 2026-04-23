#pragma once

#ifdef __cplusplus
extern "C" {
#endif

#include <stdint.h>

void search(const char *text, const char **buffer, uint16_t max_out);

#ifdef __cplusplus
}
#endif
