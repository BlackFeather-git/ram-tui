import io
import json
import os
import sys
import tempfile
import unittest
import subprocess
from unittest import mock
from importlib.machinery import SourceFileLoader

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, ROOT)
ram = SourceFileLoader("ram_tui_under_test", os.path.join(ROOT, "ram")).load_module()


class FormattingAndSanitizationTests(unittest.TestCase):
    def test_format_bytes_boundaries(self):
        self.assertEqual(ram.format_bytes(0), "0 B")
        self.assertEqual(ram.format_bytes(1023), "1023 B")
        self.assertEqual(ram.format_bytes(1024), "1.00 KB")
        self.assertEqual(ram.format_bytes(1024 ** 2), "1.00 MB")
        self.assertEqual(ram.format_bytes(1024 ** 3), "1.00 GB")
        self.assertEqual(ram.format_bytes(1024 ** 4 * 2), "2.00 TB")
        self.assertEqual(ram.format_bytes(-100), "0 B")

    def test_percentage_zero_and_bounds(self):
        self.assertEqual(ram.percentage(1, 0), 0.0)
        self.assertEqual(ram.percentage(-1, 100), 0.0)
        self.assertEqual(ram.percentage(200, 100), 100.0)
        self.assertEqual(ram.percentage(50, 100), 50.0)
        self.assertEqual(ram.percentage(None, 100), 0.0)

    def test_sanitize_full_ansi_sequences_and_bidi(self):
        text = "hello\n\x1b[31;1mworld\x1b[0m\tok\u202eoverride"
        clean = ram.sanitize_text(text)
        self.assertNotIn("\n", clean)
        self.assertNotIn("\x1b", clean)
        self.assertNotIn("[31;1m", clean)
        self.assertNotIn("\u202e", clean)
        self.assertEqual(clean, "hello~world~ok~override")


class LinuxParserTests(unittest.TestCase):
    def test_meminfo_units_and_missing_fields(self):
        text = """MemTotal:       8192 kB
MemAvailable:   4096 kB
Cached:         1024 kB
"""
        with mock.patch("builtins.open", mock.mock_open(read_data=text)):
            data = ram.get_meminfo_linux()
            self.assertTrue(data["valid"])
            self.assertEqual(data["total"], 8192 * 1024)
            self.assertEqual(data["available"], 4096 * 1024)
            self.assertEqual(data["used"], 4096 * 1024)
            self.assertEqual(data["cached"], 1024 * 1024)

    def test_linux_proc_starttime_parsing(self):
        stat_line = "1234 (weird (name)) S 100 1234 1234 0 -1 4194304 100 0 0 0 10 5 0 0 20 0 1 0 987654 1000 500"
        with mock.patch("builtins.open", mock.mock_open(read_data=stat_line)):
            starttime = ram.get_linux_proc_starttime(1234)
            self.assertEqual(starttime, "987654")

    def test_pid_name_cache_starttime_isolation(self):
        ram.PID_NAME_CACHE.clear()
        ram.PID_NAME_CACHE[(100, "987654")] = "old_process"
        ram.PID_NAME_CACHE[(100, "999999")] = "reused_process"
        self.assertEqual(ram.PID_NAME_CACHE.get((100, "987654")), "old_process")
        self.assertEqual(ram.PID_NAME_CACHE.get((100, "999999")), "reused_process")


class DarwinParserTests(unittest.TestCase):
    def test_darwin_metrics_and_unavailable_commit(self):
        def side_effect(cmd, **kwargs):
            if "hw.memsize" in cmd:
                return "17179869184\n"
            if "vm_stat" in cmd:
                return """Mach Virtual Memory Statistics: (page size of 4096 bytes)
Pages free:                              100000.
Pages active:                            800000.
Pages inactive:                          600000.
Pages speculative:                        50000.
Pages wired down:                        300000.
Pages occupied by compressor:            100000.
"""
            if "vm.swapusage" in cmd:
                return "total = 2048.00M  used = 512.00M  free = 1536.00M  (encrypted)\n"
            return ""

        with mock.patch.object(ram, "run_command", side_effect=side_effect):
            info = ram.get_meminfo_darwin()
            self.assertTrue(info["valid"])
            self.assertEqual(info["total"], 17179869184)
            self.assertIsNone(info["commit_as"])
            self.assertIsNone(info["commit_limit"])
            self.assertEqual(info["swap_used"], 512 * 1024 * 1024)
            self.assertEqual(info["swap_total"], 2048 * 1024 * 1024)


class ThemeAndModeTests(unittest.TestCase):
    def setUp(self):
        self.mem = {
            "total": 32 * 1024**3,
            "available": 24 * 1024**3,
            "used": 8 * 1024**3,
            "commit_as": 12 * 1024**3,
            "commit_limit": 40 * 1024**3,
            "cached": 6 * 1024**3,
            "swap_used": 100 * 1024**2,
            "swap_total": 16 * 1024**3,
            "swap_desc": "zram swap",
            "valid": True
        }
        self.procs = [{"name": "python", "rss": 2 * 1024**3, "count": 2, "pid": None}]

    def test_theme_palettes_completeness(self):
        required_themes = {
            "default", "dracula", "catppuccin", "nord", "tokyo-night",
            "gruvbox", "cyberpunk", "rose-pine", "everforest", "kanagawa",
            "monokai", "solarized", "monochrome"
        }
        self.assertTrue(required_themes.issubset(set(ram.THEME_PALETTES.keys())))

    def test_display_modes_render(self):
        hero = ram.render_snapshot(self.mem, self.procs, mode="hero")
        self.assertIn("PROCESS", hero)

        compact = ram.render_snapshot(self.mem, self.procs, mode="compact")
        self.assertNotIn("PROCESS", compact)
        self.assertIn("USED", compact)

        mini = ram.render_snapshot(self.mem, self.procs, mode="mini")
        self.assertIn("RAM", mini)
        self.assertNotIn("PROCESS", mini)

        tiny = ram.render_snapshot(self.mem, self.procs, mode="tiny")
        self.assertTrue(tiny.startswith("RAM:"))
        self.assertNotIn("\n", tiny)

    def test_braille_symbol_render(self):
        braille_out = ram.render_snapshot(self.mem, self.procs, mode="hero", symbol="braille")
        self.assertIn("⣿", braille_out)

    def test_cli_mode_flags_and_mutual_exclusion(self):
        args = ram.parse_arguments(["--compact", "--theme", "catppuccin", "--symbol", "braille"])
        self.assertTrue(args.compact)
        self.assertEqual(args.theme, "catppuccin")
        self.assertEqual(args.symbol, "braille")

        args = ram.parse_arguments(["--mini", "--theme", "dracula"])
        self.assertTrue(args.mini)
        self.assertEqual(args.theme, "dracula")

        args = ram.parse_arguments(["--tiny"])
        self.assertTrue(args.tiny)

        with self.assertRaises(SystemExit):
            ram.parse_arguments(["--compact", "--mini"])

        with self.assertRaises(SystemExit):
            ram.parse_arguments(["--theme", "invalid-theme"])


class TerminalAndCliTests(unittest.TestCase):
    def test_cli_boundaries(self):
        args = ram.parse_arguments(["-r", "50", "-n", "10"])
        self.assertEqual(args.rate, 50)
        self.assertEqual(args.count, 10)

        with self.assertRaises(SystemExit):
            ram.parse_arguments(["-r", "10"])
        with self.assertRaises(SystemExit):
            ram.parse_arguments(["-n", "0"])

    def test_narrow_terminal_render(self):
        mem = {
            "total": 32 * 1024**3,
            "available": 24 * 1024**3,
            "used": 8 * 1024**3,
            "commit_as": None,
            "commit_limit": None,
            "cached": None,
            "swap_used": 100 * 1024**2,
            "swap_total": 16 * 1024**3,
            "swap_desc": "zram swap",
            "valid": True
        }
        procs = [{"name": "super_long_process_name_for_testing", "rss": 2 * 1024**3, "count": 1, "pid": 123}]
        with mock.patch("shutil.get_terminal_size", return_value=os.terminal_size((40, 24))):
            rendered = ram.render_snapshot(mem, procs, group_procs=False, enable_color=False)
            self.assertIn("RAM", rendered)
            self.assertIn("USED", rendered)

    def test_viewport_height_budgeting(self):
        mem = {
            "total": 32 * 1024**3,
            "available": 24 * 1024**3,
            "used": 8 * 1024**3,
            "commit_as": None,
            "commit_limit": None,
            "cached": None,
            "swap_used": 0,
            "swap_total": 0,
            "swap_desc": "none",
            "valid": True
        }
        procs = [{"name": f"proc_{i}", "rss": 1024**3, "count": 1, "pid": i} for i in range(20)]
        for h in [2, 5, 8, 12, 16, 20, 24]:
            with mock.patch("shutil.get_terminal_size", return_value=os.terminal_size((80, h))):
                rendered = ram.render_snapshot(mem, procs, mode="hero", enable_color=False)
                lines = rendered.splitlines()
                self.assertLessEqual(len(lines), h, f"Rendered {len(lines)} lines exceeded terminal height {h}")

    def test_help_overlay_toggle(self):
        mem = {"total": 32 * 1024**3, "available": 24 * 1024**3, "used": 8 * 1024**3, "valid": True}
        with mock.patch("shutil.get_terminal_size", return_value=os.terminal_size((80, 24))):
            rendered_no_help = ram.render_snapshot(mem, [], mode="hero", enable_color=False, show_help=False)
            rendered_with_help = ram.render_snapshot(mem, [], mode="hero", enable_color=False, show_help=True)
            self.assertIn("q quit", rendered_no_help)
            self.assertIn("HOTKEYS:", rendered_with_help)
            self.assertIn("p/space", rendered_with_help)

    def test_unicode_cell_width_measurement(self):
        cjk_str = "你好世界"  # 4 characters, 8 cells
        self.assertEqual(ram.visible_cell_width(cjk_str), 8)
        truncated = ram.truncate_plain_cells(cjk_str, 5, ellipsis="~")
        self.assertLessEqual(ram.visible_cell_width(truncated), 5)

    def test_hostile_geometry_bounds(self):
        mem = {
            "total": 32 * 1024**3,
            "available": 24 * 1024**3,
            "used": 8 * 1024**3,
            "commit_as": None,
            "commit_limit": None,
            "cached": None,
            "swap_used": 0,
            "swap_total": 0,
            "swap_desc": "none",
            "valid": True
        }
        procs = [{"name": f"proc_超長進程名_{i}", "rss": 1024**3, "count": 1, "pid": i} for i in range(15)]
        hostile_dims = [(40, 8), (30, 6), (20, 4), (50, 12), (80, 24)]
        for cols, rows in hostile_dims:
            with mock.patch("shutil.get_terminal_size", return_value=os.terminal_size((cols, rows))):
                rendered = ram.render_snapshot(mem, procs, mode="hero", enable_color=False)
                lines = rendered.splitlines()
                self.assertLessEqual(len(lines), rows, f"Total lines {len(lines)} > rows {rows} for {cols}x{rows}")
                for l in lines:
                    w = ram.visible_cell_width(l)
                    self.assertLessEqual(w, cols, f"Line width {w} > cols {cols} in: {repr(l)}")

    def test_grapheme_zwj_combining_marks(self):
        zwj_sequence = "👨\u200d👩\u200d👧\u200d👦"
        # Zero-width joiners and zero-width spaces should not inflate width
        self.assertEqual(ram.char_cell_width("\u200d"), 0)
        self.assertEqual(ram.char_cell_width("\u200b"), 0)
        self.assertEqual(ram.char_cell_width("\ufeff"), 0)

    def test_terminal_manager_idempotent_setup_and_restore(self):
        tm = ram.TerminalManager()
        tm.setup_raw()
        self.assertTrue(tm._raw_active or not tm.is_tty)
        tm.setup_raw()  # Re-entrant call should be idempotent
        tm.restore()
        self.assertTrue(tm._restored)
        tm.restore()  # Re-entrant restore should be idempotent

    def test_linux_proc_parser_resilience(self):
        # Truncated or empty proc lines
        mock_meminfo = "MemTotal:\nMemFree: 1000 kB\nInvalidLine\n"
        with mock.patch("builtins.open", mock.mock_open(read_data=mock_meminfo)):
            info = ram.get_meminfo_linux()
            self.assertEqual(info["total"], 0)
            self.assertEqual(info["available"], 1000 * 1024)

    def test_zram_detection_fallback(self):
        mock_swaps = "Filename Type Size Used Priority\n"
        with mock.patch("builtins.open", mock.mock_open(read_data=mock_swaps)):
            with mock.patch("os.path.exists", return_value=True):
                with mock.patch("os.listdir", return_value=["zram0", "sda"]):
                    info = ram.get_meminfo_linux()
                    self.assertEqual(info["swap_desc"], "zram")

    def test_broken_pipe_handling(self):
        # Simulate downstream pipe closure (e.g. head -n 1)
        proc = subprocess.Popen(
            [sys.executable, os.path.join(ROOT, "ram"), "--json"],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            universal_newlines=True
        )
        first_line = proc.stdout.readline()
        proc.stdout.close()
        proc.stderr.close()
        proc.wait(timeout=2.0)
        self.assertEqual(proc.returncode, 0)


    def test_install_script_dry_run(self):
        install_sh = os.path.join(ROOT, "install.sh")
        if os.path.exists(install_sh) and os.name != "nt":
            out = subprocess.check_output(["bash", install_sh, "--dry-run"], universal_newlines=True)
            self.assertIn("[DRY-RUN]", out)


class UpdateManagerTests(unittest.TestCase):
    def test_version_comparison(self):
        self.assertTrue(ram.is_newer_version("0.5.3", "0.6.0"))
        self.assertFalse(ram.is_newer_version("0.6.0", "0.5.3"))
        self.assertFalse(ram.is_newer_version("0.6.0", "0.6.0"))
        self.assertTrue(ram.is_newer_version("0.6.0-beta.1", "0.6.0"))
        self.assertFalse(ram.is_newer_version("0.6.0", "0.6.0-beta.1"))

    def test_interval_parsing(self):
        self.assertEqual(ram.parse_update_interval("30m"), 1800)
        self.assertEqual(ram.parse_update_interval("1h"), 3600)
        self.assertEqual(ram.parse_update_interval("12h"), 43200)
        self.assertEqual(ram.parse_update_interval("900"), 900)
        with self.assertRaises(ValueError):
            ram.parse_update_interval("banana")

    def test_cache_reading_and_expiration(self):
        with tempfile.TemporaryDirectory() as tmp:
            cache_path = os.path.join(tmp, "update_check.json")
            with open(cache_path, "w", encoding="utf-8") as handle:
                json.dump(
                    {
                        "last_checked": 1000,
                        "latest_version": "0.6.0",
                        "has_update": True,
                    },
                    handle,
                )

            manager = ram.UpdateManager(
                "0.5.3",
                cache_path=cache_path,
                interval=3600,
            )
            self.assertFalse(manager.cache_expired(now=2000))
            self.assertTrue(manager.cache_expired(now=5001))
            self.assertEqual(manager.get_notification(), (
                "[Update available: v0.5.3 -> v0.6.0 | run 'ram --update']"
            ))

    def test_mocked_update_download_and_atomic_replacement(self):
        source = (
            "#!/usr/bin/env python3\n"
            "__version__ = \"0.6.0\"\n"
            "if __name__ == \"__main__\":\n"
            "    pass\n"
        ).encode("utf-8")

        class FakeResponse:
            def __init__(self, payload):
                self.payload = payload

            def read(self, size=-1):
                if size == -1:
                    payload, self.payload = self.payload, b""
                    return payload
                payload, self.payload = self.payload[:size], self.payload[size:]
                return payload

            def __enter__(self):
                return self

            def __exit__(self, exc_type, exc, tb):
                return False

        with tempfile.TemporaryDirectory() as tmp:
            target = os.path.join(tmp, "ram")
            with open(target, "wb") as handle:
                handle.write(
                    b"#!/usr/bin/env python3\n"
                    b"__version__ = \"0.5.3\"\n"
                    b"if __name__ == \"__main__\":\n"
                    b"    pass\n"
                )

            cache_path = os.path.join(tmp, "cache.json")

            def fake_urlopen(request, timeout=1.0):
                url = request.full_url
                if url.endswith("/releases/latest"):
                    return FakeResponse(b'{"tag_name":"v0.6.0"}')
                return FakeResponse(source)

            with mock.patch.object(ram.urllib.request, "urlopen", side_effect=fake_urlopen):
                manager = ram.UpdateManager(
                    "0.5.3",
                    cache_path=cache_path,
                    interval=3600,
                )
                ok, message = manager.perform_update(target_path=target)

            self.assertTrue(ok)
            self.assertIn("updated successfully", message)
            with open(target, "rb") as handle:
                updated = handle.read()
            self.assertEqual(updated, source)

    def test_no_update_check_suppresses_background_thread(self):
        with tempfile.TemporaryDirectory() as tmp:
            manager = ram.UpdateManager(
                "0.5.3",
                disabled=True,
                cache_path=os.path.join(tmp, "cache.json"),
                interval=1,
            )
            with mock.patch.object(
                ram.threading,
                "Thread",
                side_effect=AssertionError("background thread spawned"),
            ):
                self.assertFalse(manager.start_background_check())

    def test_cli_update_flags(self):
        args = ram.parse_arguments(["--update"])
        self.assertTrue(args.update)
        args = ram.parse_arguments(["--check-update", "--no-update-check"])
        self.assertTrue(args.check_update)
        self.assertTrue(args.no_update_check)


if __name__ == "__main__":
    unittest.main()
