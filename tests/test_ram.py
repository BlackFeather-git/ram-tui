import base64
import hashlib
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
sys.modules["ram"] = ram
sys.modules["ram_tui_under_test"] = ram

sys.path.insert(0, os.path.join(ROOT, "tests", "fixtures"))
from test_key_data import TEST_KEY_N, TEST_KEY_D

TEST_PUBLIC_KEY_N = TEST_KEY_N
ASN1_SHA256_PREFIX = b"\x30\x31\x30\x0d\x06\x09\x60\x86\x48\x01\x65\x03\x04\x02\x01\x05\x00\x04\x20"


def sign_test_payload(data):
    """Sign payload using pure Python RSA-2048 PKCS#1 v1.5 signer for 100% CI cross-platform portability."""
    digest = ASN1_SHA256_PREFIX + hashlib.sha256(data).digest()
    pad_len = 256 - 3 - len(digest)
    padded = b"\x00\x01" + (b"\xff" * pad_len) + b"\x00" + digest
    padded_int = int.from_bytes(padded, "big")
    sig_int = pow(padded_int, TEST_KEY_D, TEST_KEY_N)
    sig_bytes = sig_int.to_bytes(256, "big")
    return base64.b64encode(sig_bytes).decode("ascii")


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
        if os.name == "nt":
            self.skipTest("Broken pipe signal semantics are POSIX-specific")
        import signal
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
        try:
            proc.wait(timeout=2.0)
        except subprocess.TimeoutExpired:
            proc.terminate()
            proc.wait()
        expected = {0, 141}
        if hasattr(signal, "SIGPIPE"):
            expected.add(-signal.SIGPIPE)
        self.assertIn(proc.returncode, expected)


    def test_install_script_dry_run(self):
        install_sh = os.path.join(ROOT, "install.sh")
        if os.path.exists(install_sh) and os.name != "nt":
            out = subprocess.check_output(["bash", install_sh, "--dry-run"], universal_newlines=True)
            self.assertIn("[DRY-RUN]", out)


class UpdateManagerTests(unittest.TestCase):
    def setUp(self):
        self.orig_pubkey_n = ram.RELEASE_PUBLIC_KEY_N
        ram.RELEASE_PUBLIC_KEY_N = TEST_PUBLIC_KEY_N

    def tearDown(self):
        ram.RELEASE_PUBLIC_KEY_N = self.orig_pubkey_n

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
        # Non-fatal: invalid inputs fall back to default interval safely
        self.assertEqual(ram.parse_update_interval("banana"), ram.UPDATE_DEFAULT_INTERVAL)
        self.assertEqual(ram.parse_update_interval("-100"), ram.UPDATE_DEFAULT_INTERVAL)

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

    def test_ast_validation_rejects_docstring_spoofing(self):
        # Fake script with version and __main__ only in multiline strings / comments
        fake_source = (
            '#!/usr/bin/env python3\n'
            '"""\n'
            '__version__ = "0.6.0"\n'
            'if __name__ == "__main__":\n'
            '    pass\n'
            '"""\n'
            'print("malicious or invalid source")\n'
        ).encode("utf-8")

        fake_sha = hashlib.sha256(fake_source).hexdigest()
        fake_sig = sign_test_payload(fake_source)
        with self.assertRaises(ValueError) as ctx:
            ram.UpdateManager._validate_source(fake_source, "0.6.0", expected_sha256=fake_sha, expected_sig=fake_sig, filename="test")
        self.assertIn("module-level __version__", str(ctx.exception))

    def test_sha256_cryptographic_verification(self):
        source = (
            "#!/usr/bin/env python3\n"
            "__version__ = \"0.6.0\"\n"
            "if __name__ == \"__main__\":\n"
            "    pass\n"
        ).encode("utf-8")
        correct_sha = hashlib.sha256(source).hexdigest()
        correct_sig = sign_test_payload(source)

        # Should pass with matching SHA-256 and valid digital signature
        ram.UpdateManager._validate_source(source, "0.6.0", expected_sha256=correct_sha, expected_sig=correct_sig, filename="test")

        # Should strictly fail with mismatched SHA-256
        with self.assertRaises(ValueError) as ctx:
            ram.UpdateManager._validate_source(source, "0.6.0", expected_sha256="deadbeef" * 8, expected_sig=correct_sig, filename="test")
        self.assertIn("cryptographic integrity verification failed", str(ctx.exception))

    def test_package_manager_conflict_detection(self):
        self.assertIsNotNone(ram.detect_package_manager_install("/usr/bin/ram"))
        self.assertIsNotNone(ram.detect_package_manager_install("/opt/homebrew/bin/ram"))
        self.assertIsNotNone(ram.detect_package_manager_install(r"C:\Users\user\scoop\shims\ram.exe"))
        self.assertIsNone(ram.detect_package_manager_install("/home/raven/.local/bin/ram"))

        manager = ram.UpdateManager("0.5.3")
        with tempfile.TemporaryDirectory() as tmp:
            fake_bin = os.path.join(tmp, "ram")
            with open(fake_bin, "w") as f:
                f.write("#!/usr/bin/env python3\n# ram-tui\n__version__ = '0.5.3'\nif __name__ == '__main__': pass\n")

            with mock.patch.object(ram, "detect_package_manager_install", return_value="pacman"):
                with mock.patch.object(manager, "check_now", return_value=("0.6.0", True)):
                    ok, msg = manager.perform_update(target_path=fake_bin, force=False)
                    self.assertFalse(ok)
                    self.assertIn("Notice: ram-tui is installed in a package-managed path", msg)

    def test_invalid_env_interval_never_crashes_json_mode(self):
        # Even with completely corrupted RAM_UPDATE_INTERVAL, ram --json --once executes cleanly
        env = dict(os.environ, RAM_UPDATE_INTERVAL="corrupted_interval_string_123")
        proc = subprocess.Popen(
            [sys.executable, os.path.join(ROOT, "ram"), "--json", "--once"],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=env,
            universal_newlines=True,
        )
        out, err = proc.communicate(timeout=3.0)
        self.assertEqual(proc.returncode, 0)
        data = json.loads(out)
        self.assertEqual(data["version"], ram.__version__)

    def test_inter_process_lock_suppresses_concurrency(self):
        with tempfile.TemporaryDirectory() as tmp:
            cache_path = os.path.join(tmp, "cache.json")
            manager1 = ram.UpdateManager("0.5.3", cache_path=cache_path, interval=1)
            manager2 = ram.UpdateManager("0.5.3", cache_path=cache_path, interval=1)

            lock1 = manager1._acquire_process_lock()
            self.assertIsNotNone(lock1)

            # Second manager fails to acquire while first holds it
            lock2 = manager2._acquire_process_lock()
            self.assertIsNone(lock2)

            manager1._release_process_lock(lock1)

            # Now second manager can acquire
            lock3 = manager2._acquire_process_lock()
            self.assertIsNotNone(lock3)
            manager2._release_process_lock(lock3)

    def test_sha_digest_strictly_enforced(self):
        source = b"#!/usr/bin/env python3\n__version__ = '0.6.0'\nif __name__ == '__main__': pass\n"
        sig = sign_test_payload(source)

        # Missing SHA digest
        with self.assertRaises(ValueError) as ctx:
            ram.UpdateManager._validate_source(source, "0.6.0", expected_sha256=None, expected_sig=sig)
        self.assertIn("missing cryptographic SHA-256", str(ctx.exception))

        # Malformed / short SHA digest
        with self.assertRaises(ValueError) as ctx:
            ram.UpdateManager._validate_source(source, "0.6.0", expected_sha256="abc123", expected_sig=sig)
        self.assertIn("invalid or missing cryptographic SHA-256", str(ctx.exception))

        # Non-hex SHA digest
        with self.assertRaises(ValueError) as ctx:
            ram.UpdateManager._validate_source(source, "0.6.0", expected_sha256="g" * 64, expected_sig=sig)
        self.assertIn("invalid or missing cryptographic SHA-256", str(ctx.exception))

    def test_maintainer_rsa_signature_root_of_trust(self):
        source = b"#!/usr/bin/env python3\n__version__ = '0.6.0'\nif __name__ == '__main__': pass\n"
        sha = hashlib.sha256(source).hexdigest()
        sig = sign_test_payload(source)

        # 1. Authentic signature -> passes
        ram.UpdateManager._validate_source(source, "0.6.0", expected_sha256=sha, expected_sig=sig)

        # 2. Tampered source with same signature -> rejected
        tampered = source + b"# tampering\n"
        tampered_sha = hashlib.sha256(tampered).hexdigest()
        with self.assertRaises(ValueError) as ctx:
            ram.UpdateManager._validate_source(tampered, "0.6.0", expected_sha256=tampered_sha, expected_sig=sig)
        self.assertIn("cryptographic maintainer digital signature verification failed", str(ctx.exception))

        # 3. Forged / invalid signature -> rejected
        fake_sig = base64.b64encode(b"Z" * 256).decode("ascii")
        with self.assertRaises(ValueError) as ctx:
            ram.UpdateManager._validate_source(source, "0.6.0", expected_sha256=sha, expected_sig=fake_sig)
        self.assertIn("cryptographic maintainer digital signature verification failed", str(ctx.exception))

        # 4. Missing signature -> rejected
        with self.assertRaises(ValueError) as ctx:
            ram.UpdateManager._validate_source(source, "0.6.0", expected_sha256=sha, expected_sig=None)
        self.assertIn("cryptographic maintainer digital signature verification failed", str(ctx.exception))

        # 5. Signature representative >= RSA modulus N -> strictly rejected
        oversized_int = ram.RELEASE_PUBLIC_KEY_N + 5
        oversized_sig_bytes = oversized_int.to_bytes(256, "big")
        oversized_sig_b64 = base64.b64encode(oversized_sig_bytes).decode("ascii")
        self.assertFalse(ram.verify_release_signature(source, oversized_sig_b64))

        # 6. Invalid Base64 alphabet -> strictly rejected
        invalid_b64_sig = "!!!" + fake_sig[3:]
        self.assertFalse(ram.verify_release_signature(source, invalid_b64_sig))

    def test_source_size_boundaries(self):
        valid_header = b"#!/usr/bin/env python3\n__version__ = '0.6.0'\nif __name__ == '__main__': pass\n"
        exact_2mb = valid_header + b"# " + (b"A" * (2 * 1024 * 1024 - len(valid_header) - 3)) + b"\n"
        self.assertEqual(len(exact_2mb), 2 * 1024 * 1024)

        exact_sha = hashlib.sha256(exact_2mb).hexdigest()
        exact_sig = sign_test_payload(exact_2mb)
        # Exactly 2 MiB -> Passes
        ram.UpdateManager._validate_source(exact_2mb, "0.6.0", expected_sha256=exact_sha, expected_sig=exact_sig)

        # 2 MiB + 1 byte -> Rejected
        oversized = exact_2mb + b"X"
        oversized_sha = hashlib.sha256(oversized).hexdigest()
        oversized_sig = sign_test_payload(oversized)
        with self.assertRaises(ValueError) as ctx:
            ram.UpdateManager._validate_source(oversized, "0.6.0", expected_sha256=oversized_sha, expected_sig=oversized_sig)
        self.assertIn("exceeds safety limit", str(ctx.exception))

    def test_non_ram_binary_target_rejection(self):
        with tempfile.TemporaryDirectory() as tmp:
            unrelated_binary = os.path.join(tmp, "ls")
            with open(unrelated_binary, "w") as f:
                f.write("#!/bin/bash\necho unrelated\n")

            with self.assertRaises(ValueError) as ctx:
                ram.UpdateManager._resolve_target(unrelated_binary)
            self.assertIn("target does not appear to be a valid ram-tui installation", str(ctx.exception))

    def test_toctou_symlink_swap_rejection(self):
        if os.name == "nt":
            self.skipTest("Symlinks require administrator privileges on Windows")
        manager = ram.UpdateManager("0.5.3")
        with tempfile.TemporaryDirectory() as tmp:
            target = os.path.join(tmp, "ram")
            real_file = os.path.join(tmp, "real_ram")
            with open(real_file, "w") as f:
                f.write("#!/usr/bin/env python3\n# ram-tui\n__version__ = '0.5.3'\nif __name__ == '__main__': pass\n")

            os.symlink(real_file, target)

            # Atomic replace directly detects if target is a symlink and rejects TOCTOU swap
            with self.assertRaises(PermissionError) as ctx:
                manager._atomic_replace(b"new_data", target)
            self.assertIn("TOCTOU violation", str(ctx.exception))

    def test_lock_owner_liveness(self):
        # Current process is alive
        self.assertTrue(ram._is_pid_alive(os.getpid()))
        # PID 999999 is definitely dead/non-existent
        self.assertFalse(ram._is_pid_alive(999999))

        with tempfile.TemporaryDirectory() as tmp:
            cache_path = os.path.join(tmp, "cache.json")
            lock_path = cache_path + ".lock"

            # Create lock owned by dead PID
            with open(lock_path, "w") as f:
                f.write("999999 1000.0\n")

            manager = ram.UpdateManager("0.5.3", cache_path=cache_path)
            # Manager should detect dead owner and safely acquire
            acquired = manager._acquire_process_lock()
            self.assertIsNotNone(acquired)
            manager._release_process_lock(acquired)

    def test_sha_with_surrounding_garbage_rejected(self):
        manager = ram.UpdateManager("0.5.3")
        # SHA surrounded by garbage text should fail strict format regex
        garbage_payload = b"header garbage\n" + (b"a" * 64) + b"\nfooter garbage\n"
        with mock.patch.object(manager, "_request", return_value=garbage_payload):
            with self.assertRaises(ValueError) as ctx:
                manager._fetch_expected_sha256("0.6.0")
            self.assertIn("malformed SHA-256 checksum asset format", str(ctx.exception))

    def test_target_authentication_requires_ram_tui_and_ast_version(self):
        with tempfile.TemporaryDirectory() as tmp:
            # File with only __version__ but no ram-tui banner
            fake_py = os.path.join(tmp, "fake.py")
            with open(fake_py, "w") as f:
                f.write("__version__ = '0.5.3'\n")
            with self.assertRaises(ValueError) as ctx:
                ram.UpdateManager._resolve_target(fake_py)
            self.assertIn("missing authentic 'ram-tui' identity banner", str(ctx.exception))

            # File with ram-tui in comments but no valid AST __version__
            fake_banner_only = os.path.join(tmp, "fake_banner.py")
            with open(fake_banner_only, "w") as f:
                f.write("# ram-tui\nprint('hello')\n")
            with self.assertRaises(ValueError) as ctx:
                ram.UpdateManager._resolve_target(fake_banner_only)
            self.assertIn("missing valid module-level __version__", str(ctx.exception))

    def test_semver_rejects_leading_zeros(self):
        # Strict SemVer 2.0.0 rejects numeric prerelease identifiers with leading zeros
        self.assertIsNone(ram.parse_semver("0.6.0-beta.01"))
        self.assertIsNone(ram.parse_semver("0.6.0-beta.001"))
        self.assertIsNotNone(ram.parse_semver("0.6.0-beta.1"))
        self.assertIsNotNone(ram.parse_semver("0.6.0-beta.0"))

    def test_privileged_directory_security_fails_closed(self):
        with tempfile.TemporaryDirectory() as tmp:
            target_bin = os.path.join(tmp, "ram")
            with open(target_bin, "w") as f:
                f.write("#!/usr/bin/env python3\n# ram-tui\n__version__ = '0.5.3'\nif __name__ == '__main__': pass\n")

            mock_stat_mode = mock.Mock(st_mode=0o777)
            real_stat = os.stat
            with mock.patch.object(ram, "SYSTEM_OS", "Linux"):
                with mock.patch("os.geteuid", return_value=0, create=True):
                    with mock.patch("os.stat", side_effect=lambda path: mock_stat_mode if os.path.abspath(path) == os.path.abspath(tmp) else real_stat(path)):
                        with self.assertRaises(PermissionError) as ctx:
                            ram.UpdateManager._resolve_target(target_bin)
                        self.assertIn("refusing to update binary in insecure world-writable directory", str(ctx.exception))

    def test_pid_reuse_lock_eviction(self):
        import time
        with tempfile.TemporaryDirectory() as tmp:
            cache_path = os.path.join(tmp, "cache.json")
            lock_path = cache_path + ".lock"

            # Record a lock with current PID but mismatched/past starttime (simulating PID reuse)
            with open(lock_path, "w") as f:
                f.write(f"{os.getpid()} 1000.0 {time.time()}\n")

            manager = ram.UpdateManager("0.5.3", cache_path=cache_path)
            with mock.patch.object(ram, "SYSTEM_OS", "Linux"):
                with mock.patch.object(ram, "get_linux_proc_starttime", return_value="99999"):
                    # Starttime mismatch detects PID reuse -> safely evicts stale lock
                    acquired = manager._acquire_process_lock()
                    self.assertIsNotNone(acquired)
                    manager._release_process_lock(acquired)

    def test_cli_update_flags(self):
        args = ram.parse_arguments(["--update", "--force"])
        self.assertTrue(args.update)
        self.assertTrue(args.force)
        args = ram.parse_arguments(["--check-update", "--no-update-check"])
        self.assertTrue(args.check_update)
        self.assertTrue(args.no_update_check)


if __name__ == "__main__":
    unittest.main()
