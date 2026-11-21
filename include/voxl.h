#ifndef VOXL_H
#define VOXL_H
/* C API of libvoxl (src/ffi.rs). Frames use voxl's browser wire format: a 16-byte header
 * (0x01, flags, frame id, pipeline latency, stripe info) followed by H.264. */
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif

#define VOXL_OK 0
#define VOXL_ERR_ARG (-1)
#define VOXL_ERR_STATE (-2)
#define VOXL_ERR_INIT (-3)
#define VOXL_FRAME_VIDEO 1
#define VOXL_FRAME_TEXT 2

typedef struct VoxlSession VoxlSession;

typedef struct VoxlConfig {
  const char *display; /* NULL: default */
  const char *preset;  /* NULL: "ultrafast" */
  uint32_t fps;        /* 0 for any numeric field: default */
  uint32_t max_kbps;
  uint32_t min_kbps;
  uint32_t threads;
  float crf;
  uint8_t enable_resize;
} VoxlConfig;

typedef void (*VoxlFrameCallback)(const uint8_t *data, size_t len, uint32_t kind, void *user_data);

VoxlSession *voxl_session_create(const VoxlConfig *config);
int32_t voxl_session_start(VoxlSession *s, VoxlFrameCallback cb, void *user_data);
int32_t voxl_session_stop(VoxlSession *s);
int32_t voxl_session_send_mouse(VoxlSession *s, int32_t x, int32_t y, uint32_t button_mask, int32_t wheel);
int32_t voxl_session_send_key(VoxlSession *s, uint32_t keysym, int32_t down);
int32_t voxl_session_resize(VoxlSession *s, uint32_t width, uint32_t height);
void voxl_session_destroy(VoxlSession *s);

#ifdef __cplusplus
}
#endif
#endif
