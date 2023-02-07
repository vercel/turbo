#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct Buffer {
  uint32_t len;
  uint8_t *data;
} Buffer;

struct Buffer get_turbo_data_dir(void);

struct Buffer npm_transitive_closure(struct Buffer buf);

struct Buffer npm_subgraph(struct Buffer buf);
