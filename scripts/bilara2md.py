#!/usr/bin/env python3
import json
import os
import re
import sys


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
      - whether this key’s formatting opens a paragraph (contains "<p>")
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
        # Join all entries’ parts with a space.
        paragraph = " ".join(" ".join(entry["parts"]) for entry in entries)
        if paragraph:
            result_lines.append(paragraph)
            result_lines.append("")
    elif mode == "blockquote":
        # Determine if the first entry explicitly included a blockquote.
        explicit = entries[0]["explicit_blockquote"]
        if explicit:
            # Simply output every text fragment with "> " prefix.
            for entry in entries:
                for part in entry["parts"]:
                    if part:
                        result_lines.append(f"> {part}")
        else:
            # Otherwise, if more than one entry is grouped, output the first entry,
            # then a blank blockquote line, then the remaining entries.
            if len(entries) == 1:
                for part in entries[0]["parts"]:
                    if part:
                        result_lines.append(f"> {part}")
            else:
                for part in entries[0]["parts"]:
                    if part:
                        result_lines.append(f"> {part}")
                result_lines.append(">")  # blank blockquote line
                for entry in entries[1:]:
                    for part in entry["parts"]:
                        if part:
                            result_lines.append(f"> {part}")
        # result_lines.append("")
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
        to the same “paragraph” (that is, if the formatting template shows an opening <p>
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
        src, sub = key.split(":")
        splits = sub.split(".")
        group, seq = splits[0], splits[1]
        return (src, int(group), int(seq))

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


def transform_all_in_folder(source_folder: str, format_folder: str, target_folder: str):
    sutta_name_regex = re.compile(r"([a-z]+[0-9]+(\.[0-9]+)?)")

    # read file by-file in source_folder and format_folder
    os.makedirs(target_folder, exist_ok=True)

    for source_file in os.listdir(source_folder):
        sutta_name = sutta_name_regex.match(source_file).group(0)
        format_file_name = f"{sutta_name}_html.json"
        source_path = os.path.join(source_folder, source_file)
        format_path = os.path.join(format_folder, format_file_name)
        target_path = os.path.join(
            target_folder, os.path.splitext(source_file)[0] + ".md"
        )

        markdown = transform_sutta(source_path, format_path)

        with open(target_path, "w") as f:
            f.write(markdown)


def main():
    transform_all_in_folder(sys.argv[1], sys.argv[2], "output_sc_md")


if __name__ == "__main__":
    main()
