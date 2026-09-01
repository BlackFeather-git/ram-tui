/*
 * ram_tui_min.c
 * Minimal single-file C prototype of ram-tui (Linux-only, POSIX/C99).
 * Build: gcc -O2 -std=c99 ram_tui_min.c -o ram-tui-min-c
 */

#define _POSIX_C_SOURCE 200809L
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <dirent.h>
#include <ctype.h>
#include <termios.h>
#include <signal.h>
#include <time.h>
#include <sys/select.h>
#include <stdbool.h>
#include <stdint.h>

#define MAX_PROCS 256
#define NAME_MAX_LEN 32

typedef struct {
    uint64_t total_kb;
    uint64_t available_kb;
    uint64_t used_kb;
    uint64_t cached_kb;
    uint64_t swap_total_kb;
    uint64_t swap_used_kb;
} MemInfo;

typedef struct {
    int pid;
    char name[NAME_MAX_LEN];
    uint64_t rss_kb;
} ProcInfo;

typedef struct {
    const char *name;
    uint8_t accent_r, accent_g, accent_b;
} Theme;

static Theme THEMES[] = {
    {"default", 180, 120, 255},
    {"solar",   255, 180, 0},
    {"mint",    80,  220, 180}
};
static const int THEME_COUNT = 3;

static struct termios orig_termios;
static bool termios_active = false;

static void restore_terminal(void) {
    if (termios_active) {
        tcsetattr(STDIN_FILENO, TCSAFLUSH, &orig_termios);
        printf("\033[?25h\033[0m\n");
        fflush(stdout);
        termios_active = false;
    }
}

static void sig_handler(int sig) {
    (void)sig;
    restore_terminal();
    exit(0);
}

static void enable_raw_mode(void) {
    if (!isatty(STDIN_FILENO)) return;
    if (tcgetattr(STDIN_FILENO, &orig_termios) == -1) return;
    struct termios raw = orig_termios;
    raw.c_lflag &= ~(ECHO | ICANON);
    raw.c_cc[VMIN] = 0;
    raw.c_cc[VTIME] = 0;
    tcsetattr(STDIN_FILENO, TCSAFLUSH, &raw);
    termios_active = true;
    atexit(restore_terminal);
    signal(SIGINT, sig_handler);
    signal(SIGTERM, sig_handler);
    printf("\033[?25l"); /* Hide cursor */
    fflush(stdout);
}

static bool read_meminfo(MemInfo *out) {
    FILE *f = fopen("/proc/meminfo", "r");
    if (!f) return false;

    memset(out, 0, sizeof(*out));
    char line[256];
    uint64_t free_kb = 0, buffers_kb = 0, swap_free_kb = 0;
    bool has_avail = false;

    while (fgets(line, sizeof(line), f)) {
        char key[64];
        uint64_t val = 0;
        if (sscanf(line, "%63[^:]: %lu", key, &val) == 2) {
            if (strcmp(key, "MemTotal") == 0) out->total_kb = val;
            else if (strcmp(key, "MemFree") == 0) free_kb = val;
            else if (strcmp(key, "MemAvailable") == 0) { out->available_kb = val; has_avail = true; }
            else if (strcmp(key, "Cached") == 0) out->cached_kb = val;
            else if (strcmp(key, "Buffers") == 0) buffers_kb = val;
            else if (strcmp(key, "SwapTotal") == 0) out->swap_total_kb = val;
            else if (strcmp(key, "SwapFree") == 0) swap_free_kb = val;
        }
    }
    fclose(f);

    if (!has_avail) {
        out->available_kb = free_kb + buffers_kb + out->cached_kb;
    }
    if (out->total_kb > out->available_kb) {
        out->used_kb = out->total_kb - out->available_kb;
    }
    if (out->swap_total_kb > swap_free_kb) {
        out->swap_used_kb = out->swap_total_kb - swap_free_kb;
    }
    return out->total_kb > 0;
}

static int compare_procs(const void *a, const void *b) {
    const ProcInfo *pa = (const ProcInfo *)a;
    const ProcInfo *pb = (const ProcInfo *)b;
    if (pb->rss_kb > pa->rss_kb) return 1;
    if (pb->rss_kb < pa->rss_kb) return -1;
    return 0;
}

static size_t read_processes(ProcInfo *list, size_t max_count) {
    DIR *d = opendir("/proc");
    if (!d) return 0;

    long page_size_kb = sysconf(_SC_PAGESIZE) / 1024;
    if (page_size_kb <= 0) page_size_kb = 4;

    size_t count = 0;
    struct dirent *de;
    while ((de = readdir(d)) != NULL) {
        if (!isdigit(de->d_name[0])) continue;
        int pid = atoi(de->d_name);
        if (pid <= 0) continue;

        char path[64];
        snprintf(path, sizeof(path), "/proc/%d/statm", pid);
        FILE *f = fopen(path, "r");
        if (!f) continue;

        unsigned long dummy = 0, pages = 0;
        if (fscanf(f, "%lu %lu", &dummy, &pages) != 2 || pages == 0) {
            fclose(f);
            continue;
        }
        fclose(f);

        uint64_t rss_kb = pages * page_size_kb;
        if (rss_kb == 0) continue;

        char name[NAME_MAX_LEN] = {0};
        snprintf(path, sizeof(path), "/proc/%d/comm", pid);
        f = fopen(path, "r");
        if (f) {
            if (fgets(name, sizeof(name), f)) {
                size_t len = strlen(name);
                while (len > 0 && (name[len-1] == '\n' || name[len-1] == '\r')) {
                    name[--len] = '\0';
                }
            }
            fclose(f);
        }
        if (name[0] == '\0') strncpy(name, "unknown", sizeof(name) - 1);

        list[count].pid = pid;
        strncpy(list[count].name, name, NAME_MAX_LEN - 1);
        list[count].rss_kb = rss_kb;
        count++;

        if (count >= MAX_PROCS) break;
    }
    closedir(d);

    qsort(list, count, sizeof(ProcInfo), compare_procs);
    if (count > max_count) count = max_count;
    return count;
}

static void format_human(uint64_t kb, char *buf, size_t buf_len) {
    double b = (double)kb * 1024.0;
    if (b >= 1024.0 * 1024.0 * 1024.0) {
        snprintf(buf, buf_len, "%.2f GB", b / (1024.0 * 1024.0 * 1024.0));
    } else if (b >= 1024.0 * 1024.0) {
        snprintf(buf, buf_len, "%.1f MB", b / (1024.0 * 1024.0));
    } else if (b >= 1024.0) {
        snprintf(buf, buf_len, "%.0f KB", b / 1024.0);
    } else {
        snprintf(buf, buf_len, "%lu B", (unsigned long)b);
    }
}

static void render_tui(const MemInfo *mem, const ProcInfo *procs, size_t proc_count, const Theme *th, bool symbol_mode) {
    time_t now = time(NULL);
    struct tm *tm_info = localtime(&now);
    char time_str[16];
    strftime(time_str, sizeof(time_str), "%H:%M:%S", tm_info);

    char used_s[32], avail_s[32], total_s[32], cached_s[32], swap_s[32];
    format_human(mem->used_kb, used_s, sizeof(used_s));
    format_human(mem->available_kb, avail_s, sizeof(avail_s));
    format_human(mem->total_kb, total_s, sizeof(total_s));
    format_human(mem->cached_kb, cached_s, sizeof(cached_s));
    format_human(mem->swap_used_kb, swap_s, sizeof(swap_s));

    double used_pct = mem->total_kb > 0 ? ((double)mem->used_kb / (double)mem->total_kb) * 100.0 : 0.0;

    printf("\033[H");
    printf("\033[1mRAM-TUI (C Native Prototype)\033[0m\n");
    printf("shadow - Linux x86_64 - %s\n\n", time_str);

    printf("%.1f%%  %s used of %s\n\n", used_pct, used_s, total_s);
    printf("USED         AVAILABLE    TOTAL\n");
    printf("%-12s %-12s %-12s\n\n", used_s, avail_s, total_s);

    printf("COMMIT       CACHED       SWAP\n");
    printf("%-12s %-12s %-12s\n\n", used_s, cached_s, swap_s);

    printf("PROCESS (RESIDENT SET)\n");
    size_t display_count = proc_count > 8 ? 8 : proc_count;
    uint64_t total_display = 0;
    for (size_t i = 0; i < display_count; i++) total_display += procs[i].rss_kb;
    if (total_display == 0) total_display = 1;

    for (size_t i = 0; i < display_count; i++) {
        char rss_s[32];
        format_human(procs[i].rss_kb, rss_s, sizeof(rss_s));
        printf("%-20s (%d) %10s\n", procs[i].name, procs[i].pid, rss_s);
    }

    printf("\nUSAGE (bar graph)\n");
    for (size_t i = 0; i < display_count; i++) {
        double pct = ((double)procs[i].rss_kb / (double)total_display) * 100.0;
        int bar_len = (int)((pct / 100.0) * 30.0 + 0.5);
        printf("%-16s %5.1f%% \033[38;2;%d;%d;%dm", procs[i].name, pct, th->accent_r, th->accent_g, th->accent_b);
        for (int b = 0; b < bar_len; b++) {
            fputs(symbol_mode ? "█" : "#", stdout);
        }
        printf("\033[0m\n");
    }

    printf("\nq quit  p pause  t theme  s symbol  +/- rate  h help\n");
    fflush(stdout);
}

static void emit_json(const MemInfo *mem, const ProcInfo *procs, size_t proc_count) {
    printf("{\"total_kb\":%lu,\"available_kb\":%lu,\"used_kb\":%lu,\"swap_used_kb\":%lu,\"processes\":[",
           (unsigned long)mem->total_kb, (unsigned long)mem->available_kb,
           (unsigned long)mem->used_kb, (unsigned long)mem->swap_used_kb);
    for (size_t i = 0; i < proc_count; i++) {
        printf("{\"pid\":%d,\"name\":\"%s\",\"rss_kb\":%lu}%s",
               procs[i].pid, procs[i].name, (unsigned long)procs[i].rss_kb,
               (i + 1 == proc_count) ? "" : ",");
    }
    printf("]}\n");
}

int main(int argc, char **argv) {
    bool once = false;
    bool json_mode = false;
    int rate_ms = 500;

    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "--once") == 0) once = true;
        else if (strcmp(argv[i], "--json") == 0) json_mode = true;
        else if (strcmp(argv[i], "--rate") == 0 && i + 1 < argc) {
            rate_ms = atoi(argv[++i]);
            if (rate_ms < 20) rate_ms = 20;
        }
    }

    MemInfo mem;
    ProcInfo procs[MAX_PROCS];

    if (once && json_mode) {
        if (read_meminfo(&mem)) {
            size_t count = read_processes(procs, 20);
            emit_json(&mem, procs, count);
        }
        return 0;
    }

    if (once) {
        if (read_meminfo(&mem)) {
            size_t count = read_processes(procs, 20);
            render_tui(&mem, procs, count, &THEMES[0], true);
        }
        return 0;
    }

    enable_raw_mode();

    int theme_idx = 0;
    bool symbol_mode = true;
    bool paused = false;

    while (1) {
        /* Check non-blocking stdin input */
        fd_set fds;
        FD_ZERO(&fds);
        FD_SET(STDIN_FILENO, &fds);
        struct timeval tv = {0, 0};

        if (select(STDIN_FILENO + 1, &fds, NULL, NULL, &tv) > 0) {
            char ch;
            if (read(STDIN_FILENO, &ch, 1) > 0) {
                if (ch == 'q' || ch == 3) break; /* q or Ctrl+C */
                else if (ch == 'p') paused = !paused;
                else if (ch == 't') theme_idx = (theme_idx + 1) % THEME_COUNT;
                else if (ch == 's') symbol_mode = !symbol_mode;
                else if (ch == '+' && rate_ms > 50) rate_ms -= 50;
                else if (ch == '-') rate_ms += 50;
            }
        }

        if (!paused) {
            if (read_meminfo(&mem)) {
                size_t count = read_processes(procs, 20);
                render_tui(&mem, procs, count, &THEMES[theme_idx], symbol_mode);
            }
        }

        struct timespec req = { rate_ms / 1000, (rate_ms % 1000) * 1000000L };
        nanosleep(&req, NULL);
    }

    restore_terminal();
    return 0;
}
