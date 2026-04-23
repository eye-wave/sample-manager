#pragma once

typedef struct Node {
  struct Node *next[37];
  const char **output;
  unsigned short len;
} Node;
