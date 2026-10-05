/* Owned binary protocol, not a redeclaration of the Swiss Ephemeris ABI.
 * Every integer has an explicit little-endian representation. Doubles travel
 * via memcpy to uint64_t: neither decimal formatting nor pointer punning is
 * permitted. All buffers/counts are bounded before native dispatch. */
#ifndef PARITY_WIRE_H
#define PARITY_WIRE_H
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <float.h>

#define PARITY_MAGIC UINT32_C(0x53575031)
#define MAX_FRAME 16384
#define MAX_INTS 512
#define MAX_FLOATS 1024
#define MAX_TEXTS 256
#define TEXT_BYTES 4096

_Static_assert(sizeof(double) == 8 && DBL_MANT_DIG == 53, "binary64 required");
_Static_assert(sizeof(int) == 4, "native int width");

/* All transport storage is initialized. Counts, not implicit sentinels,
 * determine which portions are transmitted. Native text remains raw bytes. */
typedef struct {
    uint32_t id, op, ni, nf, nt;
    int32_t ints[32];
    double floats[64];
    char texts[4][1025];
    int present[4];
} Request;
typedef struct {
    uint32_t id, op, ni, nf, nt, text_used;
    int32_t code, ints[MAX_INTS];
    double floats[MAX_FLOATS];
    uint32_t lengths[MAX_TEXTS];
    unsigned char texts[TEXT_BYTES];
} Result;
typedef struct { unsigned char bytes[MAX_FRAME]; size_t pos, len; } Frame;

/* Protocol failures terminate the isolated worker. No binary frame/value is
 * printed: the driver reports the active input recipe instead. */
static void fail(const char *reason) { fprintf(stderr, "C oracle protocol: %s\n", reason); exit(2); }
static void require(int valid, const char *reason) { if (!valid) fail(reason); }
static uint32_t get_u32(Frame *frame) {
    require(frame->pos <= frame->len && frame->len - frame->pos >= 4, "truncated integer");
    const unsigned char *b = frame->bytes + frame->pos; frame->pos += 4;
    return (uint32_t)b[0] | (uint32_t)b[1] << 8 | (uint32_t)b[2] << 16 | (uint32_t)b[3] << 24;
}
static double get_float(Frame *frame) {
    require(frame->pos <= frame->len && frame->len - frame->pos >= 8, "truncated double");
    uint64_t bits = 0;
    for (unsigned i = 0; i < 8; ++i) bits |= (uint64_t)frame->bytes[frame->pos++] << (8 * i);
    double value; memcpy(&value, &bits, sizeof(value)); return value;
}
static void put_u32(Frame *frame, uint32_t value) {
    require(frame->pos + 4 <= MAX_FRAME, "output capacity");
    for (unsigned i = 0; i < 4; ++i) frame->bytes[frame->pos++] = (unsigned char)(value >> (8 * i));
}
static void put_float(Frame *frame, double value) {
    require(frame->pos + 8 <= MAX_FRAME, "output capacity");
    uint64_t bits; memcpy(&bits, &value, sizeof(bits));
    for (unsigned i = 0; i < 8; ++i) frame->bytes[frame->pos++] = (unsigned char)(bits >> (8 * i));
}
static int read_request(Request *request) {
    unsigned char length[4];
    size_t first = fread(length, 1, 1, stdin);
    if (first == 0) { require(!ferror(stdin), "input read"); return 0; }
    require(fread(length + 1, 1, 3, stdin) == 3, "truncated frame length");
    uint32_t n = (uint32_t)length[0] | (uint32_t)length[1] << 8 | (uint32_t)length[2] << 16 | (uint32_t)length[3] << 24;
    require(n > 0 && n <= MAX_FRAME, "frame size");
    Frame frame = {{0}, 0, n};
    require(fread(frame.bytes, 1, n, stdin) == n, "truncated frame");
    memset(request, 0, sizeof(*request));
    require(get_u32(&frame) == PARITY_MAGIC, "protocol version");
    request->id = get_u32(&frame); request->op = get_u32(&frame);
    request->ni = get_u32(&frame); request->nf = get_u32(&frame); request->nt = get_u32(&frame);
    require(request->ni <= 32 && request->nf <= 64 && request->nt <= 4, "request counts");
    for (unsigned i = 0; i < request->ni; ++i) request->ints[i] = (int32_t)get_u32(&frame);
    for (unsigned i = 0; i < request->nf; ++i) request->floats[i] = get_float(&frame);
    for (unsigned i = 0; i < request->nt; ++i) {
        uint32_t len = get_u32(&frame);
        if (len == UINT32_MAX) continue;
        require(len <= 1024 && frame.pos <= frame.len && frame.len - frame.pos >= len, "text size");
        request->present[i] = 1;
        memcpy(request->texts[i], frame.bytes + frame.pos, len); frame.pos += len;
    }
    require(frame.pos == frame.len, "trailing request data"); return 1;
}
static void add_int(Result *result, int32_t value) {
    require(result->ni < MAX_INTS, "integer capacity"); result->ints[result->ni++] = value;
}
static void add_float(Result *result, double value) {
    require(result->nf < MAX_FLOATS, "double capacity"); result->floats[result->nf++] = value;
}
static void add_floats(Result *result, const double *values, size_t length) {
    for (size_t i = 0; i < length; ++i) add_float(result, values[i]);
}
static void add_text(Result *result, const char *bytes, size_t length) {
    require(result->nt < MAX_TEXTS && length <= TEXT_BYTES - result->text_used, "text capacity");
    result->lengths[result->nt++] = (uint32_t)length;
    memcpy(result->texts + result->text_used, bytes, length); result->text_used += (uint32_t)length;
}
/* The capacity belongs to the actual native allocation. A missing NUL does
 * not license an over-read and is reported as the full bounded byte string. */
static void add_bounded_text(Result *result, const char *text, size_t capacity) {
    size_t n = 0; while (n < capacity && text[n] != 0) ++n; add_text(result, text, n);
}
static void write_result(const Result *result) {
    Frame frame = {{0}, 0, 0};
    put_u32(&frame, PARITY_MAGIC); put_u32(&frame, result->id); put_u32(&frame, result->op);
    put_u32(&frame, (uint32_t)result->code); put_u32(&frame, result->ni); put_u32(&frame, result->nf); put_u32(&frame, result->nt);
    for (unsigned i = 0; i < result->ni; ++i) put_u32(&frame, (uint32_t)result->ints[i]);
    for (unsigned i = 0; i < result->nf; ++i) put_float(&frame, result->floats[i]);
    size_t offset = 0;
    for (unsigned i = 0; i < result->nt; ++i) {
        size_t n = result->lengths[i]; put_u32(&frame, (uint32_t)n);
        require(frame.pos + n <= MAX_FRAME, "text frame capacity");
        memcpy(frame.bytes + frame.pos, result->texts + offset, n); frame.pos += n; offset += n;
    }
    Frame prefix = {{0}, 0, 0}; put_u32(&prefix, (uint32_t)frame.pos);
    require(fwrite(prefix.bytes, 1, 4, stdout) == 4 && fwrite(frame.bytes, 1, frame.pos, stdout) == frame.pos && fflush(stdout) == 0, "output write");
}
#endif
