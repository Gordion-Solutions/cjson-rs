/* Parse/print benchmark for any library that provides cJSON's API.
 *
 *   cbench <file>            best-of-5 timings, one line per operation
 *   cbench --dump <file>     parse, then write the unformatted print to stdout
 *   cbench --dump-pretty <file>
 *
 * Link it with -lcjson against upstream cJSON or cjson-rs; the same source runs
 * against both. */
#include <cJSON.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#define RUNS 5

static double now(void)
{
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (double)ts.tv_sec + (double)ts.tv_nsec * 1e-9;
}

static char *slurp(const char *path, size_t *len)
{
    FILE *f = fopen(path, "rb");
    if (!f) { perror(path); exit(2); }
    fseek(f, 0, SEEK_END);
    long n = ftell(f);
    fseek(f, 0, SEEK_SET);
    char *buf = malloc((size_t)n + 1);
    if (!buf || fread(buf, 1, (size_t)n, f) != (size_t)n) { fprintf(stderr, "read failed\n"); exit(2); }
    buf[n] = 0;
    fclose(f);
    *len = (size_t)n;
    return buf;
}

static void report(const char *op, double best, double mb)
{
    printf("%-22s %8.3f s %9.1f MB/s\n", op, best, mb / best);
}

int main(int argc, char **argv)
{
    int dump = 0, pretty = 0;
    int a = 1;
    if (argc > 2 && !strcmp(argv[1], "--dump")) { dump = 1; a = 2; }
    else if (argc > 2 && !strcmp(argv[1], "--dump-pretty")) { dump = 1; pretty = 1; a = 2; }
    if (a >= argc) { fprintf(stderr, "usage: cbench [--dump|--dump-pretty] <file>\n"); return 2; }

    size_t len;
    char *text = slurp(argv[a], &len);
    double mb = (double)len / 1048576.0;

    if (dump) {
        cJSON *j = cJSON_Parse(text);
        if (!j) { fprintf(stderr, "parse failed\n"); return 1; }
        char *s = pretty ? cJSON_Print(j) : cJSON_PrintUnformatted(j);
        if (!s) { fprintf(stderr, "print failed\n"); return 1; }
        fputs(s, stdout);
        free(s);
        cJSON_Delete(j);
        return 0;
    }

    double best_parse = 1e9, best_free = 1e9, best_print = 1e9, best_pretty = 1e9;
    size_t printed = 0;
    for (int r = 0; r < RUNS; r++) {
        double t = now();
        cJSON *j = cJSON_Parse(text);
        double tp = now() - t;
        if (!j) { fprintf(stderr, "parse failed\n"); return 1; }
        if (tp < best_parse) best_parse = tp;

        t = now();
        char *s = cJSON_PrintUnformatted(j);
        double tu = now() - t;
        if (!s) { fprintf(stderr, "print failed\n"); return 1; }
        printed = strlen(s);
        free(s);
        if (tu < best_print) best_print = tu;

        t = now();
        s = cJSON_Print(j);
        double tf = now() - t;
        if (!s) { fprintf(stderr, "pretty print failed\n"); return 1; }
        free(s);
        if (tf < best_pretty) best_pretty = tf;

        t = now();
        cJSON_Delete(j);
        double td = now() - t;
        if (td < best_free) best_free = td;
    }
    printf("input %.1f MB, unformatted output %.1f MB\n", mb, (double)printed / 1048576.0);
    report("parse", best_parse, mb);
    report("print (unformatted)", best_print, mb);
    report("print (formatted)", best_pretty, mb);
    report("delete", best_free, mb);
    return 0;
}
