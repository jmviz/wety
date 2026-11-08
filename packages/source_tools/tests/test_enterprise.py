import os
import stat
import tempfile
import time
import unittest
from pathlib import Path
from unittest import mock

from source_tools import enterprise


class TokenFile(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.path = Path(self.dir.name) / "enterprise.env"
        self.env = mock.patch.dict(os.environ, {"WETY_ENTERPRISE_ENV": str(self.path)})
        self.env.start()

    def tearDown(self):
        self.env.stop()
        self.dir.cleanup()

    def test_written_private_and_read_back(self):
        enterprise.write_env(self.path, {"WME_USERNAME": "someone", "WME_ACCESS_TOKEN": "a=b"})
        self.assertEqual(stat.S_IMODE(self.path.stat().st_mode), 0o600)
        self.assertEqual(enterprise.read_env(self.path), {"WME_USERNAME": "someone", "WME_ACCESS_TOKEN": "a=b"})

    def test_fresh_token_needs_no_request(self):
        later = int(time.time()) + 86000
        enterprise.write_env(self.path, {"WME_ACCESS_TOKEN": "tok", "WME_ACCESS_EXPIRES": later})
        with mock.patch.object(enterprise, "post", side_effect=AssertionError("no request expected")):
            self.assertEqual(enterprise.access_token(), "tok")

    def test_expiring_token_is_refreshed(self):
        now = int(time.time())
        enterprise.write_env(
            self.path,
            {
                "WME_USERNAME": "someone",
                "WME_ACCESS_TOKEN": "old",
                "WME_ACCESS_EXPIRES": now + 60,
                "WME_REFRESH_TOKEN": "refresh",
                "WME_REFRESH_EXPIRES": now + 86400,
            },
        )
        with mock.patch.object(enterprise, "post", return_value={"access_token": "new", "expires_in": 86400}) as post:
            self.assertEqual(enterprise.access_token(), "new")
        post.assert_called_once_with(enterprise.AUTH + "/token-refresh", {"username": "someone", "refresh_token": "refresh"})
        self.assertEqual(enterprise.read_env(self.path)["WME_ACCESS_TOKEN"], "new")


if __name__ == "__main__":
    unittest.main()
