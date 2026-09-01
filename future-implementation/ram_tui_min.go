// ram_tui_min.go
// Minimal single-file Go prototype of ram-tui (Linux-only, std only).
// Build: go build -o ram-tui-min-go ram_tui_min.go

package main

import (
	"bufio"
	"bytes"
	"encoding/json"
	"flag"
	"fmt"
	"io/ioutil"
	"os"
	"os/exec"
	"strconv"
	"strings"
	"sync"
	"time"
)

type MemInfo struct {
	TotalKB     uint64 `json:"total_kb"`
	FreeKB      uint64 `json:"free_kb"`
	AvailableKB uint64 `json:"available_kb"`
	CachedKB    uint64 `json:"cached_kb"`
	BuffersKB   uint64 `json:"buffers_kb"`
	SwapTotalKB uint64 `json:"swap_total_kb"`
	SwapUsedKB  uint64 `json:"swap_used_kb"`
}

type ProcInfo struct {
	Pid   int    `json:"pid"`
	Name  string `json:"name"`
	RSSKB uint64 `json:"rss_kb"`
}

var themes = []struct {
	Name   string
	Accent [3]uint8
	Text   [3]uint8
	Bg     [3]uint8
}{
	{"default", [3]uint8{180, 120, 255}, [3]uint8{220, 220, 220}, [3]uint8{10, 10, 12}},
	{"solar", [3]uint8{255, 180, 0}, [3]uint8{230, 230, 230}, [3]uint8{12, 12, 20}},
	{"mint", [3]uint8{80, 220, 180}, [3]uint8{230, 230, 230}, [3]uint8{8, 10, 8}},
}

func readMeminfo() (MemInfo, error) {
	var m MemInfo
	data, err := ioutil.ReadFile("/proc/meminfo")
	if err != nil {
		return m, err
	}
	lines := bytes.Split(data, []byte{'\n'})
	values := map[string]uint64{}
	for _, line := range lines {
		parts := bytes.SplitN(line, []byte{':'}, 2)
		if len(parts) != 2 {
			continue
		}
		key := strings.TrimSpace(string(parts[0]))
		valPart := strings.Fields(string(parts[1]))
		if len(valPart) == 0 {
			continue
		}
		v, err := strconv.ParseUint(valPart[0], 10, 64)
		if err != nil {
			continue
		}
		values[key] = v
	}
	m.TotalKB = values["MemTotal"]
	m.FreeKB = values["MemFree"]
	m.AvailableKB = values["MemAvailable"]
	if m.AvailableKB == 0 {
		m.AvailableKB = m.FreeKB
	}
	m.CachedKB = values["Cached"]
	m.BuffersKB = values["Buffers"]
	m.SwapTotalKB = values["SwapTotal"]
	swapFree := values["SwapFree"]
	if m.SwapTotalKB > swapFree {
		m.SwapUsedKB = m.SwapTotalKB - swapFree
	}
	return m, nil
}

func readProcRSSKB(pid int) (uint64, error) {
	path := fmt.Sprintf("/proc/%d/statm", pid)
	data, err := ioutil.ReadFile(path)
	if err != nil {
		return 0, err
	}
	parts := strings.Fields(string(data))
	if len(parts) < 2 {
		return 0, fmt.Errorf("statm parse")
	}
	pages, err := strconv.ParseUint(parts[1], 10, 64)
	if err != nil {
		return 0, err
	}
	pageSize := uint64(os.Getpagesize())
	return (pages * pageSize) / 1024, nil
}

func readProcName(pid int) (string, error) {
	path := fmt.Sprintf("/proc/%d/comm", pid)
	data, err := ioutil.ReadFile(path)
	if err != nil {
		return "", err
	}
	name := strings.TrimSpace(string(data))
	return name, nil
}

func collectProcs(limit int) ([]ProcInfo, error) {
	dirEntries, err := ioutil.ReadDir("/proc")
	if err != nil {
		return nil, err
	}
	procs := make([]ProcInfo, 0, 64)
	for _, de := range dirEntries {
		if !de.IsDir() {
			continue
		}
		name := de.Name()
		pid, err := strconv.Atoi(name)
		if err != nil {
			continue
		}
		comm, err := readProcName(pid)
		if err != nil || comm == "" {
			continue
		}
		rss, err := readProcRSSKB(pid)
		if err != nil || rss == 0 {
			continue
		}
		procs = append(procs, ProcInfo{Pid: pid, Name: comm, RSSKB: rss})
		if len(procs) >= 1024 {
			break
		}
	}
	// sort by rss desc
	sortProcs(procs)
	if len(procs) > limit {
		procs = procs[:limit]
	}
	return procs, nil
}

func sortProcs(p []ProcInfo) {
	// simple insertion sort for small N
	for i := 1; i < len(p); i++ {
		j := i
		for j > 0 && p[j].RSSKB > p[j-1].RSSKB {
			p[j], p[j-1] = p[j-1], p[j]
			j--
		}
	}
}

func kbToHuman(kb uint64) string {
	bytes := float64(kb) * 1024.0
	const KB = 1024.0
	const MB = KB * 1024.0
	const GB = MB * 1024.0
	if bytes >= GB {
		return fmt.Sprintf("%.2f GB", bytes/GB)
	} else if bytes >= MB {
		return fmt.Sprintf("%.1f MB", bytes/MB)
	} else if bytes >= KB {
		return fmt.Sprintf("%.0f KB", bytes/KB)
	}
	return fmt.Sprintf("%.0f B", bytes)
}

const (
	CSI   = "\x1b["
	HOME  = "\x1b[H"
	RESET = "\x1b[0m"
	BOLD  = "\x1b[1m"
)

func renderTUI(mem MemInfo, procs []ProcInfo, themeIdx int, symbolMode bool) {
	var b strings.Builder
	b.WriteString(HOME)
	b.WriteString(fmt.Sprintf("%sRAM-TUI (Go Native Prototype)%s\n", BOLD, RESET))
	hostname := "shadow"
	arch := "Linux x86_64"
	now := time.Now().Format("15:04:05")
	b.WriteString(fmt.Sprintf("%s - %s - %s\n\n", hostname, arch, now))

	usedKB := uint64(0)
	if mem.TotalKB > mem.AvailableKB {
		usedKB = mem.TotalKB - mem.AvailableKB
	}
	usedPct := 0.0
	if mem.TotalKB > 0 {
		usedPct = (float64(usedKB) / float64(mem.TotalKB)) * 100.0
	}
	b.WriteString(fmt.Sprintf("%.1f%%  %s used of %s\n\n", usedPct, kbToHuman(usedKB), kbToHuman(mem.TotalKB)))
	b.WriteString("USED        AVAILABLE        TOTAL\n")
	b.WriteString(fmt.Sprintf("%-12s %-12s %-12s\n\n", kbToHuman(usedKB), kbToHuman(mem.AvailableKB), kbToHuman(mem.TotalKB)))
	b.WriteString("COMMIT      CACHED           SWAP\n")
	b.WriteString(fmt.Sprintf("%-12s %-12s %-12s\n\n", fmt.Sprintf("%.1f GB/??", float64(usedKB)/(1024.0*1024.0)), kbToHuman(mem.CachedKB), kbToHuman(mem.SwapUsedKB)))

	b.WriteString("PROCESS (RESIDENT SET)\n")
	for i, p := range procs {
		if i >= 8 {
			break
		}
		name := p.Name
		if len(name) > 20 {
			name = name[:17] + "..."
		}
		b.WriteString(fmt.Sprintf("%-20s %10s\n", fmt.Sprintf("%s (%d)", name, p.Pid), kbToHuman(p.RSSKB)))
	}

	b.WriteString("\nUSAGE (bar graph)\n")
	totalDisplay := uint64(0)
	for i := 0; i < len(procs) && i < 8; i++ {
		totalDisplay += procs[i].RSSKB
	}
	if totalDisplay == 0 {
		totalDisplay = 1
	}
	for i := 0; i < len(procs) && i < 8; i++ {
		p := procs[i]
		pct := (float64(p.RSSKB) / float64(totalDisplay)) * 100.0
		barLen := int((pct / 100.0) * 30.0)
		if barLen < 0 {
			barLen = 0
		}
		sym := "#"
		if symbolMode {
			sym = "█"
		}
		bar := strings.Repeat(sym, barLen)
		b.WriteString(fmt.Sprintf("%-20s %6.1f%% %s\n", p.Name, pct, bar))
	}

	b.WriteString("\nq quit  p pause  t theme  s symbol  +/- rate  h help\n")
	fmt.Print(b.String())
}

func emitJSON(mem MemInfo, procs []ProcInfo) {
	out := map[string]interface{}{
		"total_kb":     mem.TotalKB,
		"available_kb": mem.AvailableKB,
		"used_kb":      func() uint64 { if mem.TotalKB > mem.AvailableKB { return mem.TotalKB - mem.AvailableKB }; return 0 }(),
		"swap_used_kb": mem.SwapUsedKB,
		"processes":    procs,
	}
	enc, _ := json.MarshalIndent(out, "", "  ")
	fmt.Println(string(enc))
}

func enableSttyRaw() {
	// best-effort: use stty to set cbreak -echo
	cmd := exec.Command("sh", "-c", "stty -echo cbreak < /dev/tty")
	cmd.Stdin = os.Stdin
	_ = cmd.Run()
}

func restoreStty() {
	cmd := exec.Command("sh", "-c", "stty sane < /dev/tty")
	cmd.Stdin = os.Stdin
	_ = cmd.Run()
}

func inputReader(ch chan byte, wg *sync.WaitGroup, stop <-chan struct{}) {
	defer wg.Done()
	reader := bufio.NewReader(os.Stdin)
	for {
		select {
		case <-stop:
			return
		default:
			b, err := reader.ReadByte()
			if err != nil {
				time.Sleep(10 * time.Millisecond)
				continue
			}
			ch <- b
		}
	}
}

func main() {
	onceFlag := flag.Bool("once", false, "single snapshot and exit")
	jsonFlag := flag.Bool("json", false, "emit JSON snapshot")
	rateFlag := flag.Int("rate", 500, "sampling rate in ms")
	flag.Parse()

	if *onceFlag && *jsonFlag {
		mem, err := readMeminfo()
		if err == nil {
			procs, _ := collectProcs(20)
			emitJSON(mem, procs)
		}
		return
	}
	if *onceFlag {
		mem, err := readMeminfo()
		if err == nil {
			procs, _ := collectProcs(20)
			renderTUI(mem, procs, 0, true)
		}
		return
	}

	// Interactive
	enableSttyRaw()
	defer restoreStty()

	inputCh := make(chan byte, 64)
	stopCh := make(chan struct{})
	var wg sync.WaitGroup
	wg.Add(1)
	go inputReader(inputCh, &wg, stopCh)

	paused := false
	themeIdx := 0
	symbolMode := true
	rateMs := *rateFlag
	lastRender := time.Now().Add(-time.Duration(rateMs) * time.Millisecond)

	for {
		// handle input (non-blocking)
	loopInput:
		for {
			select {
			case b := <-inputCh:
				switch b {
				case 'q':
					close(stopCh)
					wg.Wait()
					restoreStty()
					fmt.Println("\nExiting.")
					return
				case 'p':
					paused = !paused
				case 't':
					themeIdx = (themeIdx + 1) % len(themes)
				case 's':
					symbolMode = !symbolMode
				case '+':
					if rateMs > 50 {
						rateMs -= 50
					}
				case '-':
					rateMs += 50
				case 'h':
					restoreStty()
					fmt.Println("\nHelp: q quit, p pause, t theme, s symbol, +/- rate, h help\n")
					enableSttyRaw()
				default:
					// ignore
				}
			default:
				break loopInput
			}
		}

		if paused {
			time.Sleep(100 * time.Millisecond)
			continue
		}

		if time.Since(lastRender) < time.Duration(rateMs)*time.Millisecond {
			time.Sleep(5 * time.Millisecond)
			continue
		}
		lastRender = time.Now()

		mem, err := readMeminfo()
		if err != nil {
			time.Sleep(100 * time.Millisecond)
			continue
		}
		procs, _ := collectProcs(20)
		renderTUI(mem, procs, themeIdx, symbolMode)
	}
}
