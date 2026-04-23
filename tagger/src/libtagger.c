#include "libtagger.h"
#include "gen/tags.h"
#include "node.h"

#include <stdint.h>

static uint16_t seen(const char *buffer[], uint16_t count, const char *s) {
  for (int i = 0; i < count; i++) {
    if (buffer[i] == s)
      return 1;
  }
  return 0;
}

#define MAX_STEPS 128

void search(const char *text, const char **buffer, uint16_t max_out) {
  uint16_t out_count = 0;

  for (uint16_t i = 0; text[i] != '\0'; i++) {
    uint16_t step = 0;

    Node *node = &nodes[0];

    for (uint16_t j = i; text[j] != '\0'; j++) {
      char c = text[j];

      if (++step >= MAX_STEPS) {
        break;
      }

      if (c == ' ' || c == '-' || c == '_')
        c = 26;
      else if (c >= '0' && c <= '9')
        c -= 21;
      else if (c >= 'a' && c <= 'z')
        c -= 'a';
      else
        break;

      node = node->next[c];
      if (!node)
        break;

      if (node->len > 0) {
        for (uint16_t k = 0; k < node->len; k++) {
          const char *s = node->output[k];

          if (!seen(buffer, out_count, s)) {
            if (out_count < max_out) {
              buffer[out_count++] = s;
            }
          }
        }
      }
    }
  }
}
