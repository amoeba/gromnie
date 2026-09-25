#ifndef GROMNIE_IOS_H
#define GROMNIE_IOS_H

#include <stdarg.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct gromnie_session_t gromnie_session_t;

struct gromnie_session_t *gromnie_session_create(void);

int32_t gromnie_session_connect(struct gromnie_session_t *session,
                                const char *host_utf8,
                                uint16_t port,
                                const char *username_utf8,
                                const char *password_utf8);

int32_t gromnie_session_select_character(struct gromnie_session_t *session, uint32_t character_id);

int32_t gromnie_session_send_chat(struct gromnie_session_t *session, const char *message_utf8);

int32_t gromnie_session_next_event(struct gromnie_session_t *session,
                                   uint32_t timeout_ms,
                                   uint8_t **json_utf8,
                                   size_t *json_len);

void gromnie_buffer_free(uint8_t *json_utf8, size_t json_len);

int32_t gromnie_session_disconnect(struct gromnie_session_t *session);

void gromnie_session_destroy(struct gromnie_session_t *session);

const char *gromnie_result_message(int32_t code);

#endif  /* GROMNIE_IOS_H */
