#!/usr/bin/env python3
import json
import os
import re
import sys


def parse_numeric_parts(part):
    """
    Convert an id fragment such as "33-35" into a tuple of integers that sorts
    numerically instead of lexicographically.
    """
    return tuple(int(piece) for piece in re.findall(r"\d+", part))


def split_on_j(text):
    """
    If the text contains the marker <j> (indicating a forced line break),
    split it into trimmed parts. Otherwise return a one‐element list.
    """
    if "<j>" in text:
        return [part.strip() for part in text.split("<j>")]
    return [text.strip()]


def analyze_format(fmt_template):
    """
    Examine the formatting template (a string such as:
      "<article id='sn1.1'><header><ul><li class='division'>{}</li>"
      "<p><span class='verse-line'>{}</span>"
      "{}</p>"
      etc.)
    and decide:
      - the mode: one of "header", "blockquote", "paragraph"
      - whether this key's formatting opens a paragraph (contains "<p>")
      - whether it closes one (contains "</p>")
      - whether it explicitly uses a blockquote (i.e. if it contains "<blockquote")

    (Any template that contains any of "<header", "<li", or "<h1" is treated as header;
     if it contains "verse-line" or "<blockquote" it is treated as blockquote;
     otherwise, if it contains "<p>" or "</p>" it is taken as paragraph.)
    """
    tmpl = fmt_template.lower()
    if any(tag in tmpl for tag in ("<header", "<li", "<h1")):
        mode = "header"
    elif ("verse-line" in tmpl) or ("<blockquote" in tmpl):
        mode = "blockquote"
    elif ("<p>" in tmpl) or ("</p>" in tmpl):
        mode = "paragraph"
    else:
        mode = "paragraph"
    starts = "<p>" in tmpl
    ends = "</p>" in tmpl
    explicit_blockquote = "<blockquote" in tmpl
    return mode, starts, ends, explicit_blockquote


def flush_group(group, result_lines):
    """
    Flush the current group (a dictionary with two keys: mode and entries).
    Each entry is a dict with keys:
      - "parts": list of text fragments (split on <j>)
      - "explicit_blockquote": boolean
    For mode "header" the text is output as a header line (wrapped in double underscores)
    For mode "paragraph" the text pieces are joined with a space.
    For mode "blockquote" the lines are output with a "> " prefix.

    In the special case of blockquote mode when the formatting instructions did not
    explicitly include a <blockquote> tag, we assume the first fragment should stand alone,
    and then a blank blockquote line is inserted before outputting the remaining lines.
    (This behavior reproduces, for example, the extra break seen in one sample.)
    After flushing, a blank line is appended.
    """
    if not group or not group.get("entries"):
        return

    mode = group["mode"]
    entries = group["entries"]
    if mode == "paragraph":
        # Join all entries' parts with a space.
        paragraph = " ".join(" ".join(entry["parts"]) for entry in entries)
        if paragraph:
            result_lines.append(paragraph)
            result_lines.append("")
    elif mode == "blockquote":
        # Determine if the first entry explicitly included a blockquote.
        explicit = entries[0]["explicit_blockquote"]

        # Add all verse lines with "> " prefix
        for entry in entries:
            for part in entry["parts"]:
                if part:
                    result_lines.append(f"> {part}")

        # Add an empty line after the blockquote
        result_lines.append("")
    elif mode == "header":
        # (Headers are not grouped; they are processed individually.)
        for entry in entries:
            for part in entry["parts"]:
                if part:
                    result_lines.append(f"*{part}*")
                    result_lines.append("")
    else:
        # Fallback: treat as paragraph.
        paragraph = " ".join(" ".join(entry["parts"]) for entry in entries)
        if paragraph:
            result_lines.append(paragraph)
            result_lines.append("")


def escape_braces(text):
    # Regex pattern for markdown links
    markdown_link_pattern = r"\[.*?\]\(.*?\)"

    # Find all markdown links
    markdown_links = re.findall(markdown_link_pattern, text)

    # Replace braces with escaped braces
    text = (
        text.replace("{", "\\{")
        .replace("}", "\\}")
        .replace("[", "\\[")
        .replace("]", "\\]")
        .replace("(", "\\(")
        .replace(")", "\\)")
    )

    # Replace escaped braces in markdown links with unescaped braces
    for link in markdown_links:
        escaped_link = (
            link.replace("[", "\\[")
            .replace("]", "\\]")
            .replace("(", "\\(")
            .replace(")", "\\)")
        )
        text = text.replace(escaped_link, link)

    text = (
        text.replace("`", "\\`")
        .replace("#", "\\#")
        .replace(".", "\\.")
        .replace("!", "\\!")
        .replace("=", "\\=")
        .replace("-", "\\-")
    )

    return text


def convert_json_to_markdown(json_source_str, json_format_str):
    """
    Process the source and formatting JSON documents.
    For each key (sorted in natural order) we:
      • obtain the source text and split it on any "<j>"
      • obtain the corresponding formatting template and analyze it
      • then we group consecutive entries that are of the same mode and that belong
        to the same "paragraph" (that is, if the formatting template shows an opening <p>
        then that starts a new group; if it shows a closing </p> then that group is flushed).

    The actual markdown output is produced as follows:
      - For header mode, each text fragment is wrapped with double underscores.
      - For paragraph mode, the text pieces are joined with spaces.
      - For blockquote mode, each resulting line is prefixed with "> " (and in one case an extra
        blank blockquote line is inserted if the formatting instructions did not explicitly include
        a blockquote element).

    (Note that all decisions are based solely on the formatting JSON.)
    """
    source = json.loads(json_source_str)
    formatting = json.loads(json_format_str)

    # Sort keys in natural order (we assume keys have the form "something:group.seq")
    def sort_key(key):
        src, sub = key.split(":", 1)
        group, seq = sub.split(".", 1)
        return (src, parse_numeric_parts(group), parse_numeric_parts(seq))

    keys = sorted(source.keys(), key=sort_key)

    # Build a list of entries in order. Each entry is a dict:
    # { "mode": ..., "parts": [...], "starts": bool, "ends": bool, "explicit_blockquote": bool }
    entries = []
    for key in keys:
        txt = source[key].strip()
        parts = split_on_j(txt)
        fmt_tmpl = formatting.get(key, "")
        mode, starts, ends, explicit_blockquote = analyze_format(fmt_tmpl)
        entry = {
            "mode": mode,
            "parts": parts,
            "starts": starts,
            "ends": ends,
            "explicit_blockquote": explicit_blockquote,
        }
        entries.append(entry)

    result_lines = []
    current_group = None

    # Function to flush current group and reset it.
    def flush_current():
        nonlocal current_group
        if current_group is not None:
            flush_group(current_group, result_lines)
            current_group = None

    # Process each entry in order.
    for entry in entries:
        m = entry["mode"]
        if m in ("paragraph", "blockquote"):
            # If this entry explicitly starts a new paragraph, flush the current group.
            if (
                (current_group is None)
                or (current_group["mode"] != m)
                or entry["starts"]
            ):
                flush_current()
                current_group = {"mode": m, "entries": [entry]}
            else:
                current_group["entries"].append(entry)
            if entry["ends"]:
                flush_current()
        else:  # For header (or any non–groupable mode)
            flush_current()
            # Process header immediately.
            for part in entry["parts"]:
                if part:
                    result_lines.append(f"*{part}*")
                    result_lines.append("")
    # Flush any remaining group.
    flush_current()

    return escape_braces("\n".join(result_lines))


def transform_sutta(source_path: str, format_path: str) -> str:
    try:
        with open(source_path, "r") as f:
            json_source = f.read()

        with open(format_path, "r") as f:
            json_format = f.read()

        return convert_json_to_markdown(json_source, json_format)
    except Exception as e:
        print(f"Error transforming {source_path}: {e}")
        raise


def output_filename(source_file: str, sutta_name: str, filename_format: str) -> str:
    if filename_format == "numerical":
        return sutta_name + ".md"
    # "full": preserve source stem, e.g. mn98_translation-ru-sv.json -> mn98_translation-ru-sv.md
    return os.path.splitext(source_file)[0] + ".md"


def delete_alternative(target_dir: str, source_file: str, sutta_name: str, filename_format: str):
    alt_format = "numerical" if filename_format == "full" else "full"
    alt_path = os.path.join(target_dir, output_filename(source_file, sutta_name, alt_format))
    if os.path.exists(alt_path):
        os.remove(alt_path)


def transform_all_in_folder(
    source_folder: str,
    format_folder: str,
    target_folder: str,
    filename_format: str = "full",
    overwrite: bool = False,
):
    # Match single or ranged sutta names, e.g. an5.294 or an5.294-302
    sutta_name_regex = re.compile(r"([a-z]+[0-9]+(?:\.[0-9]+)?(?:-[0-9]+)?)")

    # read file by-file in source_folder and format_folder
    os.makedirs(target_folder, exist_ok=True)

    for source_file in os.listdir(source_folder):
        match = sutta_name_regex.match(source_file)
        if not match:
            continue  # skip files that don't match the pattern
        sutta_name = match.group(1)
        format_file_name = f"{sutta_name}_html.json"
        source_path = os.path.join(source_folder, source_file)
        format_path = os.path.join(format_folder, format_file_name)
        target_path = os.path.join(
            target_folder, output_filename(source_file, sutta_name, filename_format)
        )

        if not overwrite and os.path.exists(target_path):
            continue

        if overwrite:
            delete_alternative(target_folder, source_file, sutta_name, filename_format)

        markdown = transform_sutta(source_path, format_path)

        with open(target_path, "w") as f:
            f.write(markdown)


def transform_all_recursive(
    source_folder: str,
    format_folder: str,
    target_folder: str,
    filename_format: str = "full",
    overwrite: bool = False,
):
    sutta_name_regex = re.compile(r"([a-z]+[0-9]+(?:\.[0-9]+)?(?:-[0-9]+)?)")

    os.makedirs(target_folder, exist_ok=True)

    for dirpath, _, files in os.walk(source_folder):
        rel_dir = os.path.relpath(dirpath, source_folder)
        current_format_dir = os.path.join(format_folder, rel_dir)

        matching = [f for f in files if sutta_name_regex.match(f)]
        if not matching:
            continue

        for source_file in matching:
            match = sutta_name_regex.match(source_file)
            sutta_name = match.group(1)
            format_file_name = f"{sutta_name}_html.json"
            source_path = os.path.join(dirpath, source_file)
            format_path = os.path.join(current_format_dir, format_file_name)
            target_path = os.path.join(
                target_folder, output_filename(source_file, sutta_name, filename_format)
            )

            if not overwrite and os.path.exists(target_path):
                continue

            if overwrite:
                delete_alternative(target_folder, source_file, sutta_name, filename_format)

            markdown = transform_sutta(source_path, format_path)

            with open(target_path, "w") as f:
                f.write(markdown)


def main():
    import argparse

    parser = argparse.ArgumentParser(
        description="Convert Bilara JSON suttas to Markdown"
    )
    parser.add_argument("source_folder")
    parser.add_argument("format_folder")
    parser.add_argument("target_folder")
    parser.add_argument(
        "-r",
        "--recursive",
        action="store_true",
        default=True,
        help="Traverse source and format folders recursively, writing all output flat into target_folder (default: true)",
    )
    parser.add_argument(
        "--filename-format",
        choices=["numerical", "full"],
        default="full",
        help="Output filename format: 'numerical' (e.g. sn1.22.md) or 'full' preserving the source stem (e.g. mn98_translation-ru-sv.md) (default: full)",
    )
    parser.add_argument(
        "--overwrite",
        action="store_true",
        default=False,
        help="Delete the alternative-format file for the same sutta in the target folder before writing (default: false)",
    )

    args = parser.parse_args()

    if args.recursive:
        transform_all_recursive(
            args.source_folder, args.format_folder, args.target_folder,
            args.filename_format, args.overwrite,
        )
    else:
        transform_all_in_folder(
            args.source_folder, args.format_folder, args.target_folder,
            args.filename_format, args.overwrite,
        )


if __name__ == "__main__":
    main()
