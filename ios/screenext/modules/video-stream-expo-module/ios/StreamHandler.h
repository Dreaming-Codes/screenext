#ifndef StreamHandler_h
#define StreamHandler_h

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

void ios_stream_init(void);
void ios_stream_start(void *view_handle, uint16_t port);
void ios_stream_stop(void);

#ifdef __cplusplus
}
#endif

#endif /* StreamHandler_h */
