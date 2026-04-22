#include "libtagger.h"
#include <stdio.h>
#include <string.h>

void tag_string(short *buf, char *input) {
  size_t len = strlen(input);

  for (size_t i = 0; i < 5; i++) {
    buf[i] = (short)i;
  }
}
