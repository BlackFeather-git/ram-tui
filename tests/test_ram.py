import io
import json
import os
import sys
import unittest
from unittest import mock

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, ROOT)

from importlib.machinery import SourceFileLoader
ram = SourceFileLoader("ram_tui_under_test", os.path.join(ROOT, "ram")).load_module()


class FormattingTests(unittest.TestCase):
    def test_format_bytes_boundaries(self):
        self.assertEqual(ram.format_bytes(0), "0 B")
        self.assertEqual(ram.format_bytes(1023), "1023 B")
        self.assertEqual(ram.format_bytes(1024), "1.00 KB")
        self.assertEqual(ram.format_bytes(1024 ** 2), "1.00 MB")
        self.assertEqual(ram.format_bytes(1024 ** 3), "1.00 GB")

    def test_percentage_zero_and_bounds(self):
        self.assertEqual(ram.percentage(1, 0), 0.0)
        self.assertEqual(ram.percentage(-1, 100), 0.0)
        self.assertEqual(ram.percentage(200, 100), 100.0)

    def test_sanitize_control_characters(self):
        text = "hello\n\x1b[31mworld\tok"
        clean = ram.sanitize_text(text)
        self.assertNotIn("\n", clean)
        self.assertNotIn("\x1b", clean)
        self.assertIn("world", clean)


class LinuxParserTests(unittest.TestCase):
    def test_meminfo_units_and_missing_fields(self):
        text = """MemTotal:       8192 kB
MemAvailable:   4096 kB
Cached:         1024 kB
Buffers:         128 kB
SReclaimable:    256 kB
Committed_AS:   2048 kB
CommitLimit:    8192 kB
SwapTotal:      1024 kB
SwapFree:        512 kB
"""
        info = ram.parse_linux_meminfo(text)
        self.assertEqual(info["MemTotal"], 8192 * 1024)
        self.assertEqual(info["CommitLimit"], 8192 * 1024)

        with mock.patch.object(ram, "_read_text", side_effect=[
            text, "Filename Type Size Used Priority\n/dev/zram0 partition 1024 512 -2\n"
        ]):
            snap = ram.get_meminfo_linux()

        self.assertEqual(snap["total"], 8192 * 1024)
        self.assertEqual(snap["available"], 4096 * 1024)
        self.assertEqual(snap["used"], 4096 * 1024)
        self.assertEqual(snap["swap_used"], 512 * 1024)
        self.assertEqual(snap["swap_desc"], "zram swap")

    def test_missing_memavailable_falls_back_to_memfree(self):
        text = "MemTotal: 100 kB\nMemFree: 40 kB\n"
        with mock.patch.object(ram, "_read_text", return_value=text):
            snap = ram.get_meminfo_linux()
        self.assertEqual(snap["available"], 40 * 1024)
        self.assertEqual(snap["used"], 60 * 1024)


    def test_macos_parser_uses_reported_page_size_and_swap(self):
        outputs = {
            ("sysctl", "-n", "hw.memsize"): "8589934592\n",
            ("vm_stat",): """Mach Virtual Memory Statistics: (page size of 16384 bytes)
Pages free:                               100.
Pages active:                             200.
Pages inactive:                           150.
Pages speculative:                         50.
""",
            ("sysctl", "-n", "vm.swapusage"): "total = 1024.00M  used = 128.00M  free = 896.00M\n",
        }

        def fake_command(args):
            return outputs[tuple(args)]

        with mock.patch.object(ram, "_run_command", side_effect=fake_command):
            snap = ram.get_meminfo_darwin()

        self.assertEqual(snap["total"], 8589934592)
        self.assertEqual(snap["available"], (100 + 150 + 50) * 16384)
        self.assertEqual(snap["swap_total"], 1024 * 1024 ** 2)
        self.assertEqual(snap["swap_used"], 128 * 1024 ** 2)

    def test_process_disappearing_is_ignored(self):
        with mock.patch.object(ram.os, "listdir", return_value=["123"]):
            with mock.patch.object(
                ram, "_read_text",
                side_effect=FileNotFoundError(),
            ):
                rows = ram._linux_processes()
        self.assertEqual(rows, [])


class ProcessTests(unittest.TestCase):
    def test_grouping_and_deterministic_ties(self):
        rows = [
            {"pid": 3, "name": "alpha", "rss": 100},
            {"pid": 4, "name": "alpha", "rss": 200},
            {"pid": 2, "name": "beta", "rss": 300},
            {"pid": 1, "name": "zeta", "rss": 300},
        ]
        grouped = ram.aggregate_processes(rows, True)
        self.assertEqual(grouped[0]["name"], "alpha")
        self.assertEqual(grouped[0]["rss"], 300)
        self.assertEqual(grouped[0]["count"], 2)
        self.assertEqual([x["name"] for x in grouped[1:]], ["beta", "zeta"])

        individual = ram.aggregate_processes(rows, False)
        self.assertEqual(individual[0]["pid"], 2)
        self.assertEqual(individual[1]["pid"], 1)

    def test_negative_rss_is_clamped(self):
        rows = [{"pid": 1, "name": "x", "rss": -5}]
        self.assertEqual(ram.aggregate_processes(rows)[0]["rss"], 0)


    def test_windows_memory_api_does_not_fake_pagefile_usage(self):
        class FakeKernel:
            def GlobalMemoryStatusEx(self, pointer):
                status = pointer._obj
                status.ullTotalPhys = 8 * 1024 ** 3
                status.ullAvailPhys = 3 * 1024 ** 3
                status.ullTotalPageFile = 12 * 1024 ** 3
                status.ullAvailPageFile = 4 * 1024 ** 3
                return 1

        class FakeWindll:
            kernel32 = FakeKernel()

        with mock.patch.object(ram.ctypes, "windll", FakeWindll(), create=True):
            snap = ram.get_meminfo_windows()

        self.assertEqual(snap["total"], 8 * 1024 ** 3)
        self.assertEqual(snap["used"], 5 * 1024 ** 3)
        self.assertEqual(snap["swap_total"], 4 * 1024 ** 3)
        self.assertEqual(snap["swap_used"], 0)
        self.assertIn("usage unavailable", snap["swap_desc"])


class CLITests(unittest.TestCase):
    def test_invalid_rate(self):
        with self.assertRaises(SystemExit) as exc:
            ram.parse_args(["--rate", "19"])
        self.assertNotEqual(exc.exception.code, 0)

    def test_invalid_count(self):
        with self.assertRaises(SystemExit):
            ram.parse_args(["--count", "0"])

    def test_json_payload_is_serializable(self):
        snapshot = {
            "timestamp": "2026-01-01T00:00:00+00:00",
            "hostname": "test",
            "os": "Linux",
            "version": ram.__version__,
            "memory": ram._memory_snapshot(total=100, available=40, used=60),
            "top_processes": [],
        }
        raw = json.dumps(snapshot)
        self.assertEqual(json.loads(raw)["memory"]["total"], 100)

    def test_once_does_not_require_tty(self):
        fake = {
            "timestamp": "2026-01-01T00:00:00+00:00",
            "hostname": "test",
            "os": "Linux",
            "version": ram.__version__,
            "memory": ram._memory_snapshot(total=100, available=50, used=50),
            "top_processes": [],
        }
        with mock.patch.object(ram, "build_snapshot", return_value=fake):
            stream = io.StringIO()
            with mock.patch.object(ram.sys, "stdout", stream):
                self.assertEqual(ram.main(["--once"]), 0)
        self.assertIn("RAM USAGE", stream.getvalue())


class RenderingTests(unittest.TestCase):
    def test_narrow_terminal_does_not_crash(self):
        snapshot = {
            "timestamp": "x",
            "hostname": "\x1b[31mhost",
            "os": "Linux",
            "version": ram.__version__,
            "memory": ram._memory_snapshot(total=1000, available=500, used=500),
            "top_processes": [
                {"name": "\x1b[31mbad\nname", "rss": 100, "count": 1}
            ],
        }
        with mock.patch.object(ram, "terminal_width", return_value=20):
            output = ram.render_snapshot(snapshot)
        self.assertIn("bad", output)
        self.assertNotIn("\x1b[31mbad", output)


if __name__ == "__main__":
    unittest.main()
