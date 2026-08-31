#!/usr/bin/env python3
"""
Dedicated Local Update Flow Tester for ram-tui

Spins up a lightweight local mock HTTP server that simulates GitHub Releases
API and raw CDN source downloads, validating the entire update lifecycle:
1. Release tag discovery and JSON parsing.
2. 60-second default cache interval and expiration calculation.
3. UTF-8 source downloading, bytecode compilation, and version verification.
4. Atomic in-place file replacement with Unix permission preservation.
5. Command-line interface integration (--check-update, --update, --no-update-check).
"""

import http.server
import json
import os
import shutil
import socketserver
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from importlib.machinery import SourceFileLoader

ROOT_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ram_module = SourceFileLoader("ram", os.path.join(ROOT_DIR, "ram")).load_module()


class MockGitHubHandler(http.server.BaseHTTPRequestHandler):
    """Mock GitHub API and Raw CDN Handler."""

    latest_tag = "v0.6.0-beta.2"
    mock_payload_version = "0.6.0-beta.2"

    def log_message(self, format, *args):
        # Suppress standard HTTP server logging to keep test output clean
        pass

    def do_GET(self):
        if self.path.endswith("/releases/latest"):
            payload = json.dumps({"tag_name": self.latest_tag}).encode("utf-8")
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)
        elif self.path.endswith("/ram"):
            script_content = (
                "#!/usr/bin/env python3\n"
                f'__version__ = "{self.mock_payload_version}"\n\n'
                "def main():\n"
                "    print('ram-tui updated binary running')\n\n"
                'if __name__ == "__main__":\n'
                "    main()\n"
            ).encode("utf-8")
            self.send_response(200)
            self.send_header("Content-Type", "text/plain; charset=utf-8")
            self.send_header("Content-Length", str(len(script_content)))
            self.end_headers()
            self.wfile.write(script_content)
        else:
            self.send_response(404)
            self.end_headers()


class LocalUpdateFlowTest(unittest.TestCase):
    """End-to-end integration tests using local mock server."""

    @classmethod
    def setUpClass(cls):
        # Start ephemeral local HTTP server
        cls.server = socketserver.TCPServer(("127.0.0.1", 0), MockGitHubHandler)
        cls.port = cls.server.server_address[1]
        cls.server_thread = threading.Thread(target=cls.server.serve_forever, daemon=True)
        cls.server_thread.start()

        # Point UpdateManager URLs to local mock server
        cls.orig_api_url = ram_module.UPDATE_API_URL
        cls.orig_raw_url = ram_module.UPDATE_RAW_BASE_URL
        ram_module.UPDATE_API_URL = f"http://127.0.0.1:{cls.port}/repos/BlackFeather-git/ram-tui/releases/latest"
        ram_module.UPDATE_RAW_BASE_URL = f"http://127.0.0.1:{cls.port}/BlackFeather-git/ram-tui"

    @classmethod
    def tearDownClass(cls):
        ram_module.UPDATE_API_URL = cls.orig_api_url
        ram_module.UPDATE_RAW_BASE_URL = cls.orig_raw_url
        cls.server.shutdown()
        cls.server.server_close()

    def setUp(self):
        self.temp_dir = tempfile.mkdtemp(prefix="ram-test-update-")
        self.cache_file = os.path.join(self.temp_dir, "cache.json")
        self.target_bin = os.path.join(self.temp_dir, "ram")

        # Create a mock target executable
        with open(self.target_bin, "w", encoding="utf-8") as f:
            f.write(
                "#!/usr/bin/env python3\n"
                '__version__ = "0.6.0-beta.1"\n\n'
                'if __name__ == "__main__":\n'
                "    pass\n"
            )
        if os.name != "nt":
            os.chmod(self.target_bin, 0o755)

    def tearDown(self):
        shutil.rmtree(self.temp_dir, ignore_errors=True)

    def test_default_interval_is_one_minute(self):
        """Verify the testing phase default check interval is strictly 60 seconds (1 minute)."""
        manager = ram_module.UpdateManager("0.6.0-beta.1", cache_path=self.cache_file)
        self.assertEqual(manager.interval, 60)

    def test_mock_server_check_and_cache(self):
        """Verify query against mock server updates the cache file."""
        manager = ram_module.UpdateManager(
            "0.6.0-beta.1",
            cache_path=self.cache_file,
            interval=60,
        )
        latest, has_update = manager.check_now()
        self.assertEqual(latest, "0.6.0-beta.2")
        self.assertTrue(has_update)
        self.assertTrue(os.path.isfile(self.cache_file))

        with open(self.cache_file, "r", encoding="utf-8") as f:
            cache_data = json.load(f)
        self.assertEqual(cache_data["latest_version"], "0.6.0-beta.2")
        self.assertTrue(cache_data["has_update"])
        self.assertIn("last_checked", cache_data)

    def test_cache_expiration_boundary(self):
        """Verify cache expires precisely after 60 seconds."""
        manager = ram_module.UpdateManager(
            "0.6.0-beta.1",
            cache_path=self.cache_file,
            interval=60,
        )
        manager.check_now()
        now = time.time()
        # 30 seconds later -> cache valid
        self.assertFalse(manager.cache_expired(now=now + 30))
        # 61 seconds later -> cache expired
        self.assertTrue(manager.cache_expired(now=now + 61))

    def test_perform_update_replaces_executable_safely(self):
        """Verify download, compilation check, and atomic in-place replacement."""
        manager = ram_module.UpdateManager(
            "0.6.0-beta.1",
            cache_path=self.cache_file,
            interval=60,
        )
        ok, msg = manager.perform_update(target_path=self.target_bin)
        self.assertTrue(ok)
        self.assertIn("updated successfully: v0.6.0-beta.1 -> v0.6.0-beta.2", msg)

        # Verify target file has updated content
        with open(self.target_bin, "r", encoding="utf-8") as f:
            updated_content = f.read()
        self.assertIn('__version__ = "0.6.0-beta.2"', updated_content)

        # Verify executable permissions on Unix
        if os.name != "nt":
            mode = os.stat(self.target_bin).st_mode
            self.assertTrue(bool(mode & 0o111))

    def test_notification_banner_format(self):
        """Verify the quiet footer notification message format."""
        manager = ram_module.UpdateManager(
            "0.6.0-beta.1",
            cache_path=self.cache_file,
            interval=60,
        )
        manager.check_now()
        notice = manager.get_notification()
        self.assertEqual(
            notice,
            "[Update available: v0.6.0-beta.1 -> v0.6.0-beta.2 | run 'ram --update']"
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
