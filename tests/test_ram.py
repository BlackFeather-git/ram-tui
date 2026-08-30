import io
import json
import os
import sys
import unittest
from unittest import mock
from importlib.machinery import SourceFileLoader

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, ROOT)
ram = SourceFileLoader("ram_tui_under_test", os.path.join(ROOT, "ram")).load_module()


class FormattingTests(unittest.TestCase):
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

    def test_sanitize_control_and_bidi(self):
        text = "hello\n\x1b[31mworld\tok\u202eoverride"
        clean = ram.sanitize_text(text)
        self.assertNotIn("\n", clean)
        self.assertNotIn("\x1b", clean)
        self.assertNotIn("\u202e", clean)
        self.assertIn("world", clean)


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

    def test_pid_name_cache_starttime(self):
        ram.PID_NAME_CACHE.clear()
        ram.PID_NAME_CACHE[(100, 12345)] = "old_process"
        ram.PID_NAME_CACHE[(100, 67890)] = "reused_process"
        self.assertEqual(ram.PID_NAME_CACHE.get((100, 12345)), "old_process")
        self.assertEqual(ram.PID_NAME_CACHE.get((100, 67890)), "reused_process")


class DarwinParserTests(unittest.TestCase):
    @mock.patch("subprocess.check_output")
    def test_darwin_metrics_and_unavailable_commit(self, mock_subp):
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

        mock_subp.side_effect = side_effect
        info = ram.get_meminfo_darwin()
        self.assertTrue(info["valid"])
        self.assertEqual(info["total"], 17179869184)
        self.assertIsNone(info["commit_as"])
        self.assertIsNone(info["commit_limit"])
        self.assertEqual(info["swap_used"], 512 * 1024 * 1024)
        self.assertEqual(info["swap_total"], 2048 * 1024 * 1024)


class CliAndJsonTests(unittest.TestCase):
    def test_cli_boundaries(self):
        args = ram.parse_arguments(["-r", "50", "-n", "10"])
        self.assertEqual(args.rate, 50)
        self.assertEqual(args.count, 10)

        with self.assertRaises(SystemExit):
            ram.parse_arguments(["-r", "10"])
        with self.assertRaises(SystemExit):
            ram.parse_arguments(["-n", "0"])

    def test_render_and_json_structure(self):
        mem = {
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
        procs = [{"name": "brave", "rss": 2 * 1024**3, "count": 5, "pid": None}]
        rendered = ram.render_snapshot(mem, procs, group_procs=True, enable_color=False)
        self.assertIn("RAM USAGE", rendered)
        self.assertIn("brave (5)", rendered)

        payload = {
            "timestamp": "2026-08-31T00:00:00+05:30",
            "hostname": "test-box",
            "os": "Linux",
            "version": ram.__version__,
            "memory": mem,
            "top_processes": procs
        }
        parsed = json.loads(json.dumps(payload))
        self.assertEqual(parsed["version"], "0.4.0")
        self.assertIn("memory", parsed)
        self.assertIn("top_processes", parsed)


if __name__ == "__main__":
    unittest.main()
