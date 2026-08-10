#!/usr/bin/env python3
import argparse
from pathlib import Path

from bilara2md import transform_all_recursive


def main():
    parser = argparse.ArgumentParser(
        description="Sync Bilara translations into data/ru using full filenames"
    )
    parser.add_argument("source_folder", type=Path)
    args = parser.parse_args()

    source = args.source_folder.resolve()
    if not source.is_dir():
        parser.error(f"not a directory: {source}")

    try:
        sutta_root = next(
            folder
            for folder in (source, *source.parents)
            if folder.name == "sutta" and folder.parents[2].name == "translation"
        )
    except (StopIteration, IndexError):
        parser.error("source must be inside bilara-data/translation/<lang>/<author>/sutta")

    bilara_root = sutta_root.parents[3]
    format_folder = (
        bilara_root / "html/pli/ms/sutta" / source.relative_to(sutta_root)
    )
    if not format_folder.is_dir():
        parser.error(f"matching Bilara format directory not found: {format_folder}")

    target = Path(__file__).resolve().parents[1] / "data/ru"
    transform_all_recursive(
        str(source), str(format_folder), str(target), filename_format="full", overwrite=True
    )


if __name__ == "__main__":
    main()
