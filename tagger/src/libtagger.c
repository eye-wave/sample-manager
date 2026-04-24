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

uint8_t is_boundary(char c) { return !((c >= 'a' && c <= 'z')); }

void search(const char *text, const char **buffer, uint16_t max_out) {
  uint16_t out_count = 0;

  for (uint16_t i = 0; text[i] != '\0'; i++) {

    Node *node = &nodes[0];

    for (uint16_t j = i; text[j] != '\0'; j++) {

      char c = text[j];

      if (c == ' ' || c == '-' || c == '_') {
        c = 26;
      } else if (c >= '0' && c <= '9') {
        c = c - '0' + 27;
      } else if (c >= 'a' && c <= 'z') {
        c = c - 'a';
      } else {
        break;
      }

      uint16_t next_idx = node->next[c];
      if (next_idx == 0)
        break;

      node = &nodes[next_idx];

      for (uint16_t k = 0; k < 4; k++) {
        uint16_t out_id = node->output[k];
        if (out_id == 0)
          continue;

        uint16_t start = i;
        uint16_t end = j;

        char left = (start == 0) ? ' ' : text[start - 1];
        char right = text[end + 1];

        if (node->strict) {
          if (!is_boundary(left))
            continue;
          if (right != '\0' && !is_boundary(right))
            continue;
        }

        const char *s = output_strings[out_id];

        if (!seen(buffer, out_count, s)) {
          if (out_count < max_out) {
            buffer[out_count++] = s;
          }
        }
      }
    }
  }
}
