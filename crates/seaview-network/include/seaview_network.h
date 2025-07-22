/*
 * seaview-network C API
 *
 * This header provides C bindings for the seaview-network Rust library,
 * enabling real-time mesh streaming from C/C++ applications.
 */

#ifndef SEAVIEW_NETWORK_H
#define SEAVIEW_NETWORK_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stdint.h>
#include <stddef.h>


#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * Protocol version for compatibility checking
 */
#define PROTOCOL_VERSION 2

/**
 * Maximum message size (100MB by default)
 */
#define DEFAULT_MAX_MESSAGE_SIZE ((100 * 1024) * 1024)

/**
 * Wire format options
 */
typedef enum CWireFormat {
  /**
   * Binary format (default)
   */
  Bincode = 0,
  /**
   * JSON format (if feature enabled)
   */
  Json = 1,
} CWireFormat;

/**
 * Opaque handle to a network sender
 */
typedef struct NetworkSender NetworkSender;

/**
 * Sender configuration
 */
typedef struct CSenderConfig {
  /**
   * Wire format to use
   */
  enum CWireFormat format;
  /**
   * Maximum message size in bytes
   */
  uintptr_t max_message_size;
  /**
   * Enable TCP no-delay (1 = true, 0 = false)
   */
  int tcp_nodelay;
  /**
   * Send buffer size in bytes (0 = system default)
   */
  uintptr_t send_buffer_size;
  /**
   * Connection timeout in milliseconds (0 = no timeout)
   */
  unsigned int connect_timeout_ms;
  /**
   * Write timeout in milliseconds (0 = no timeout)
   */
  unsigned int write_timeout_ms;
} CSenderConfig;

/**
 * C-compatible mesh frame structure
 */
typedef struct CMeshFrame {
  /**
   * Null-terminated simulation ID string
   */
  const char *simulation_id;
  /**
   * Frame number
   */
  unsigned int frame_number;
  /**
   * Timestamp in nanoseconds
   */
  uint64_t timestamp;
  /**
   * Domain minimum bounds (x, y, z)
   */
  float domain_min[3];
  /**
   * Domain maximum bounds (x, y, z)
   */
  float domain_max[3];
  /**
   * Number of vertices (must be divisible by 3 for triangle soup)
   */
  uintptr_t vertex_count;
  /**
   * Pointer to vertex data (x,y,z triplets)
   */
  const float *vertices;
  /**
   * Pointer to normal data (x,y,z triplets), NULL if no normals
   */
  const float *normals;
  /**
   * Number of indices, 0 if not indexed
   */
  uintptr_t index_count;
  /**
   * Pointer to index data, NULL if not indexed
   */
  const unsigned int *indices;
} CMeshFrame;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

/**
 * Create a default sender configuration
 */
struct CSenderConfig seaview_network_default_config(void);

/**
 * Create a new network sender
 *
 * # Parameters
 * - `host`: Null-terminated hostname or IP address
 * - `port`: Port number
 *
 * # Returns
 * - Pointer to NetworkSender on success
 * - NULL on failure
 */
struct NetworkSender *seaview_network_create_sender(const char *host, uint16_t port);

/**
 * Create a new network sender with custom configuration
 *
 * # Parameters
 * - `host`: Null-terminated hostname or IP address
 * - `port`: Port number
 * - `config`: Sender configuration
 *
 * # Returns
 * - Pointer to NetworkSender on success
 * - NULL on failure
 */
struct NetworkSender *seaview_network_create_sender_with_config(const char *host,
                                                                uint16_t port,
                                                                struct CSenderConfig config);

/**
 * Send a mesh frame
 *
 * # Parameters
 * - `sender`: Sender handle
 * - `mesh`: Mesh frame data
 *
 * # Returns
 * - 0 on success
 * - -1 on invalid parameters
 * - -2 on send failure
 */
int seaview_network_send_mesh(struct NetworkSender *sender, const struct CMeshFrame *mesh);

/**
 * Send a heartbeat message
 *
 * # Parameters
 * - `sender`: Sender handle
 *
 * # Returns
 * - 0 on success
 * - -1 on invalid parameters
 * - -2 on send failure
 */
int seaview_network_send_heartbeat(struct NetworkSender *sender);

/**
 * Flush any buffered data
 *
 * # Parameters
 * - `sender`: Sender handle
 *
 * # Returns
 * - 0 on success
 * - -1 on invalid parameters
 * - -2 on flush failure
 */
int seaview_network_flush(struct NetworkSender *sender);

/**
 * Get sender statistics
 *
 * # Parameters
 * - `sender`: Sender handle
 * - `frames_sent`: Pointer to store frames sent count
 * - `bytes_sent`: Pointer to store bytes sent count
 *
 * # Returns
 * - 0 on success
 * - -1 on invalid parameters
 */
int seaview_network_get_stats(struct NetworkSender *sender,
                              uint64_t *frames_sent,
                              uint64_t *bytes_sent);

/**
 * Destroy a network sender
 *
 * # Parameters
 * - `sender`: Sender handle to destroy
 */
void seaview_network_destroy_sender(struct NetworkSender *sender);

/**
 * Get the last error message
 *
 * # Returns
 * - Null-terminated error string
 * - NULL if no error
 *
 * Note: The returned string is only valid until the next FFI call
 */
const char *seaview_network_last_error(void);

/**
 * Get the library version string
 *
 * # Returns
 * - Null-terminated version string
 */
const char *seaview_network_version(void);

#ifdef __cplusplus
} // extern "C"
#endif // __cplusplus


#ifdef __cplusplus
}
#endif

#endif /* SEAVIEW_NETWORK_H */
