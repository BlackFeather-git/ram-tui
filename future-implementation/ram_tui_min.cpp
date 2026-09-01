// ram_tui_min.cpp
// Minimal single-file C++ prototype of ram-tui (Linux-only, std only).
// Build: g++ -O2 -std=c++17 ram_tui_min.cpp -o ram-tui-min-cpp

#include <algorithm>
#include <chrono>
#include <cinttypes>
#include <csignal>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <fcntl.h>
#include <iomanip>
#include <iostream>
#include <sstream>
#include <string>
#include <sys/select.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <termios.h>
#include <thread>
#include <unistd.h>
#include <vector>
#include <dirent.h>

struct MemInfo {
    uint64_t total_kb = 0;
    uint64_t free_kb = 0;
    uint64_t available_kb = 0;
    uint64_t cached_kb = 0;
    uint64_t buffers_kb = 0;
    uint64_t swap_total_kb = 0;
    uint64_t swap_used_kb = 0;
};

struct ProcInfo {
    pid_t pid = 0;
    std::string name;
    uint64_t rss_kb = 0;
};

static struct termios orig_termios;
static volatile sig_atomic_t resized = 0;

static void disable_raw_mode() {
    tcsetattr(STDIN_FILENO, TCSAFLUSH, &orig_termios);
}

static void enable_raw_mode() {
    if (tcgetattr(STDIN_FILENO, &orig_termios) == -1) return;
    struct termios raw = orig_termios;
    raw.c_lflag &= ~(ECHO | ICANON);
    raw.c_cc[VMIN] = 0;
    raw.c_cc[VTIME] = 0;
    tcsetattr(STDIN_FILENO, TCSAFLUSH, &raw);
}

static void sigint_handler(int) {
    disable_raw_mode();
    std::cout << "\nExiting.\n";
    std::exit(0);
}

static void sigwinch_handler(int) {
    (void)signal(SIGWINCH, sigwinch_handler);
    resized = 1;
}

static bool read_meminfo(MemInfo &m) {
    FILE *f = fopen("/proc/meminfo", "r");
    if (!f) return false;
    char line[512];
    while (fgets(line, sizeof(line), f)) {
        char key[64];
        uint64_t val = 0;
        if (sscanf(line, "%63[^:]: %" SCNu64, key, &val) >= 1) {
            if (strcmp(key, "MemTotal") == 0) m.total_kb = val;
            else if (strcmp(key, "MemFree") == 0) m.free_kb = val;
            else if (strcmp(key, "MemAvailable") == 0) m.available_kb = val;
            else if (strcmp(key, "Cached") == 0) m.cached_kb = val;
            else if (strcmp(key, "Buffers") == 0) m.buffers_kb = val;
            else if (strcmp(key, "SwapTotal") == 0) m.swap_total_kb = val;
            else if (strcmp(key, "SwapFree") == 0) {
                uint64_t swap_free = val;
                m.swap_used_kb = (m.swap_total_kb > swap_free) ? (m.swap_total_kb - swap_free) : 0;
            }
        }
    }
    fclose(f);
    if (m.available_kb == 0) m.available_kb = m.free_kb;
    return true;
}

static uint64_t read_proc_rss_kb(pid_t pid) {
    char path[128];
    snprintf(path, sizeof(path), "/proc/%d/statm", pid);
    FILE *f = fopen(path, "r");
    if (!f) return 0;
    unsigned long size_pages = 0, rss_pages = 0;
    if (fscanf(f, "%lu %lu", &size_pages, &rss_pages) < 2) {
        fclose(f);
        return 0;
    }
    fclose(f);
    long page = sysconf(_SC_PAGESIZE);
    if (page <= 0) page = 4096;
    uint64_t kb = (rss_pages * (uint64_t)page) / 1024;
    return kb;
}

static bool read_proc_comm(pid_t pid, std::string &out) {
    char path[128];
    snprintf(path, sizeof(path), "/proc/%d/comm", pid);
    FILE *f = fopen(path, "r");
    if (!f) return false;
    char buf[256];
    if (!fgets(buf, sizeof(buf), f)) { fclose(f); return false; }
    fclose(f);
    size_t L = strlen(buf);
    if (L && buf[L-1] == '\n') buf[L-1] = '\0';
    out = buf;
    return true;
}

static int collect_procs(std::vector<ProcInfo> &out, int limit) {
    DIR *d = opendir("/proc");
    if (!d) return 0;
    struct dirent *ent;
    std::vector<ProcInfo> tmp;
    while ((ent = readdir(d)) != nullptr) {
        if (ent->d_type != DT_DIR) continue;
        char *endptr;
        long pid = strtol(ent->d_name, &endptr, 10);
        if (*endptr != '\0') continue;
        std::string name;
        if (!read_proc_comm((pid_t)pid, name)) continue;
        uint64_t rss = read_proc_rss_kb((pid_t)pid);
        if (rss == 0) continue;
        ProcInfo p; p.pid = (pid_t)pid; p.name = std::move(name); p.rss_kb = rss;
        tmp.push_back(std::move(p));
        if ((int)tmp.size() >= 1024) break;
    }
    closedir(d);
    std::sort(tmp.begin(), tmp.end(), [](const ProcInfo &a, const ProcInfo &b){
        return a.rss_kb > b.rss_kb;
    });
    int n = std::min((int)tmp.size(), limit);
    out.assign(tmp.begin(), tmp.begin() + n);
    return n;
}

static std::string kb_to_human(uint64_t kb) {
    double bytes = (double)kb * 1024.0;
    const double KB = 1024.0;
    const double MB = KB * 1024.0;
    const double GB = MB * 1024.0;
    char buf[64];
    if (bytes >= GB) snprintf(buf, sizeof(buf), "%.2f GB", bytes / GB);
    else if (bytes >= MB) snprintf(buf, sizeof(buf), "%.1f MB", bytes / MB);
    else if (bytes >= KB) snprintf(buf, sizeof(buf), "%.0f KB", bytes / KB);
    else snprintf(buf, sizeof(buf), "%.0f B", bytes);
    return std::string(buf);
}

static void clear_home() {
    std::cout << "\x1b[H";
}

struct Theme { const char *name; int r,g,b; };

static std::vector<Theme> themes() {
    return { {"default", 180,120,255}, {"solar", 255,180,0}, {"mint", 80,220,180} };
}

static void render_tui(const MemInfo &m, const std::vector<ProcInfo> &procs, int theme_idx, bool symbol_mode) {
    clear_home();
    std::cout << "\x1b[1mRAM-TUI (C++ Native Prototype)\x1b[0m\n";
    auto now = std::chrono::system_clock::now();
    std::time_t t = std::chrono::system_clock::to_time_t(now);
    std::tm tm = *std::localtime(&t);
    std::cout << "shadow - Linux x86_64 - "
              << (tm.tm_hour < 10 ? "0" : "") << tm.tm_hour << ":"
              << (tm.tm_min < 10 ? "0" : "") << tm.tm_min << ":"
              << (tm.tm_sec < 10 ? "0" : "") << tm.tm_sec << "\n\n";

    uint64_t used_kb = (m.total_kb > m.available_kb) ? (m.total_kb - m.available_kb) : 0;
    double used_pct = (m.total_kb > 0) ? (double)used_kb / (double)m.total_kb * 100.0 : 0.0;
    std::cout.setf(std::ios::fixed); std::cout.precision(1);
    std::cout << used_pct << "%  " << kb_to_human(used_kb) << " used of " << kb_to_human(m.total_kb) << "\n\n";
    std::cout << "USED        AVAILABLE        TOTAL\n";
    std::cout << kb_to_human(used_kb) << " " << kb_to_human(m.available_kb) << " " << kb_to_human(m.total_kb) << "\n\n";
    std::cout << "COMMIT      CACHED           SWAP\n";
    std::cout << "?? " << kb_to_human(m.cached_kb) << " " << kb_to_human(m.swap_used_kb) << "\n\n";

    std::cout << "PROCESS (RESIDENT SET)\n";
    for (size_t i = 0; i < procs.size() && i < 8; ++i) {
        std::string name = procs[i].name;
        if (name.size() > 20) name = name.substr(0,17) + "...";
        std::cout << std::left << std::setw(20) << name << std::right << std::setw(12) << kb_to_human(procs[i].rss_kb) << "\n";
    }

    std::cout << "\nUSAGE (bar graph)\n";
    uint64_t total_display = 0;
    for (size_t i = 0; i < procs.size() && i < 8; ++i) total_display += procs[i].rss_kb;
    if (total_display == 0) total_display = 1;
    
    Theme th = themes()[theme_idx % themes().size()];
    for (size_t i = 0; i < procs.size() && i < 8; ++i) {
        double pct = (double)procs[i].rss_kb / (double)total_display * 100.0;
        int bar_len = (int)((pct / 100.0) * 30.0 + 0.5);
        std::string bar(bar_len, symbol_mode ? '|' : '#');
        std::cout << std::left << std::setw(20) << procs[i].name << " "
                  << std::right << std::setw(6) << std::fixed << std::setprecision(1) << pct << "% "
                  << "\033[38;2;" << th.r << ";" << th.g << ";" << th.b << "m"
                  << bar << "\033[0m\n";
    }

    std::cout << "\nq quit  p pause  t theme  s symbol  +/- rate  h help\n";
    std::cout.flush();
}

static void emit_json(const MemInfo &m, const std::vector<ProcInfo> &procs) {
    std::ostringstream o;
    o << "{";
    o << "\"total_kb\":" << m.total_kb << ",";
    o << "\"available_kb\":" << m.available_kb << ",";
    o << "\"used_kb\":" << (m.total_kb > m.available_kb ? m.total_kb - m.available_kb : 0) << ",";
    o << "\"swap_used_kb\":" << m.swap_used_kb << ",";
    o << "\"processes\":[";
    for (size_t i = 0; i < procs.size(); ++i) {
        o << "{\"pid\":" << procs[i].pid << ",\"name\":\"";
        for (char c : procs[i].name) if (c == '"') o << '\''; else o << c;
        o << "\",\"rss_kb\":" << procs[i].rss_kb << "}";
        if (i + 1 < procs.size()) o << ",";
    }
    o << "]}";
    std::cout << o.str() << std::endl;
}

int main(int argc, char **argv) {
    bool once = false, json = false;
    int rate_ms = 500;
    for (int i = 1; i < argc; ++i) {
        std::string a = argv[i];
        if (a == "--once") once = true;
        else if (a == "--json") json = true;
        else if (a == "--rate" && i + 1 < argc) { rate_ms = std::atoi(argv[++i]); if (rate_ms < 50) rate_ms = 50; }
    }

    std::signal(SIGINT, sigint_handler);
    std::signal(SIGWINCH, sigwinch_handler);

    if (once && json) {
        MemInfo m;
        if (!read_meminfo(m)) return 1;
        std::vector<ProcInfo> procs;
        collect_procs(procs, 20);
        emit_json(m, procs);
        return 0;
    }

    if (once) {
        MemInfo m;
        if (!read_meminfo(m)) return 1;
        std::vector<ProcInfo> procs;
        collect_procs(procs, 20);
        render_tui(m, procs, 0, true);
        return 0;
    }

    enable_raw_mode();
    atexit(disable_raw_mode);

    bool paused = false;
    int theme_idx = 0;
    bool symbol_mode = true;

    while (true) {
        if (resized) {
            resized = 0;
        }

        // nonblocking input
        fd_set readfds;
        FD_ZERO(&readfds);
        FD_SET(STDIN_FILENO, &readfds);
        struct timeval tv; tv.tv_sec = 0; tv.tv_usec = 0;
        int rv = select(STDIN_FILENO + 1, &readfds, NULL, NULL, &tv);
        if (rv > 0 && FD_ISSET(STDIN_FILENO, &readfds)) {
            char c;
            ssize_t r = read(STDIN_FILENO, &c, 1);
            if (r > 0) {
                if (c == 'q') { disable_raw_mode(); std::cout << "\nExiting.\n"; return 0; }
                else if (c == 'p') paused = !paused;
                else if (c == 't') theme_idx = (theme_idx + 1) % (int)themes().size();
                else if (c == 's') symbol_mode = !symbol_mode;
                else if (c == '+') { if (rate_ms > 50) rate_ms -= 50; }
                else if (c == '-') { rate_ms += 50; }
                else if (c == 'h') {
                    disable_raw_mode();
                    std::cout << "\nHelp: q quit, p pause, t theme, s symbol, +/- rate, h help\n";
                    enable_raw_mode();
                }
            }
        }

        if (paused) {
            std::this_thread::sleep_for(std::chrono::milliseconds(100));
            continue;
        }

        MemInfo m;
        if (!read_meminfo(m)) { std::this_thread::sleep_for(std::chrono::milliseconds(100)); continue; }
        std::vector<ProcInfo> procs;
        collect_procs(procs, 20);

        render_tui(m, procs, theme_idx, symbol_mode);

        std::this_thread::sleep_for(std::chrono::milliseconds(rate_ms));
    }

    return 0;
}
