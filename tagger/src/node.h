#pragma once
#include <stdint.h>

typedef struct Node {
  uint16_t next[37];
  uint16_t output[4];
  uint8_t strict;
} Node;
