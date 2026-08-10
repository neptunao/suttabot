import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import sync_suttas


class SyncSuttasTest(unittest.TestCase):
    def test_infers_matching_format_subtree(self):
        with tempfile.TemporaryDirectory() as tmp:
            bilara = Path(tmp)
            source = bilara / "translation/ru/sv/sutta/an/an3"
            formats = bilara / "html/pli/ms/sutta/an/an3"
            source.mkdir(parents=True)
            formats.mkdir(parents=True)

            with (
                patch.object(sys, "argv", ["sync_suttas.py", str(source)]),
                patch.object(sync_suttas, "transform_all_recursive") as transform,
            ):
                sync_suttas.main()

            transform.assert_called_once_with(
                str(source),
                str(formats),
                str(Path(sync_suttas.__file__).resolve().parents[1] / "data/ru"),
                filename_format="full",
                overwrite=True,
            )


if __name__ == "__main__":
    unittest.main()
