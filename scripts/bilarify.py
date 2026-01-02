import json
import os
import sys
import re
from typing import Any

# Configuration for replacements
ELLIPSIS = "…"
EM_DASH = "—"
LEVEL1_OPEN = "«"
LEVEL1_CLOSE = "»"
LEVEL2_OPEN = "„"
LEVEL2_CLOSE = "“"

class TextFormatter:
    def __init__(self):
        # Quote depth state must persist across values within a single file
        # because a sentence (and quote) might span multiple JSON keys.
        self.quote_depth = 0

    def process_string(self, text: str) -> str:
        # 1. Replace three dots with ellipsis
        text = text.replace("...", ELLIPSIS)

        # 2. Replace hyphens with em-dash, protecting intra-word hyphens
        # Step A: Identify and protect hyphens between word characters (e.g., что-нибудь)
        # We use a temporary marker that is unlikely to appear in text.
        def protect_hyphen(match):
            return match.group(1) + "__PROTECTED_HYPHEN__" + match.group(2)

        # Regex matches: (WordChar)-(WordChar)
        text = re.sub(r'(\w)-(\w)', protect_hyphen, text)

        # Step B: Replace all remaining hyphens with em-dash
        text = text.replace("-", EM_DASH)

        # Step C: Restore protected hyphens
        text = text.replace("__PROTECTED_HYPHEN__", "-")

        # 3. Process Quotes (context-sensitive)
        if '"' in text:
            text = self._replace_quotes(text)

        return text

    def _replace_quotes(self, text: str) -> str:
        # We iterate character by character to determine context (Open vs Close)
        out = []
        n = len(text)
        i = 0

        while i < n:
            char = text[i]
            if char == '"':
                # Context heuristic:
                # Open if preceded by separator (start/space) AND followed by content (word).
                # Close if preceded by content (word) AND followed by separator (end/space/punct).

                prev_char = text[i-1] if i > 0 else ' '
                next_char = text[i+1] if i < n-1 else ' '

                # We treat anything not alphanumeric as a separator boundary
                is_prev_sep = not str(prev_char).isalnum()
                is_next_sep = not str(next_char).isalnum()

                is_opening = False
                is_closing = False

                if is_prev_sep and not is_next_sep:
                    is_opening = True
                elif not is_prev_sep and is_next_sep:
                    is_closing = True
                else:
                    # Fallback for ambiguous cases (e.g., "word" with no spaces, or stray quotes)
                    # We rely on the current depth state.
                    if self.quote_depth == 0:
                        is_opening = True
                    else:
                        is_closing = True

                # Apply replacement based on state
                if is_opening:
                    if self.quote_depth == 0:
                        out.append(LEVEL1_OPEN)
                        self.quote_depth = 1
                    else:
                        # Nested quotes (Level 1+) use „
                        out.append(LEVEL2_OPEN)
                        self.quote_depth = 2
                elif is_closing:
                    self.quote_depth -= 1
                    if self.quote_depth < 0: self.quote_depth = 0 # Safety clamp

                    if self.quote_depth == 0:
                        out.append(LEVEL1_CLOSE)
                    else:
                        out.append(LEVEL2_CLOSE)
            else:
                out.append(char)
            i += 1

        return "".join(out)

def recursive_process_json(data: Any, formatter: TextFormatter) -> Any:
    """
    Recursively traverse the JSON structure (dict/list) and process strings.
    We maintain insertion order processing which is crucial for the persistent quote state.
    """
    if isinstance(data, str):
        return formatter.process_string(data)
    elif isinstance(data, list):
        return [recursive_process_json(item, formatter) for item in data]
    elif isinstance(data, dict):
        # Process values, preserve keys
        return {k: recursive_process_json(v, formatter) for k, v in data.items()}
    else:
        return data

def process_file(filepath: str):
    print(f"Processing: {filepath}")
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            data = json.load(f)

        # Initialize a new formatter for each file to reset quote depth state
        formatter = TextFormatter()
        processed_data = recursive_process_json(data, formatter)

        with open(filepath, 'w', encoding='utf-8') as f:
            # ensure_ascii=False ensures Russian characters are written as is, not escaped
            json.dump(processed_data, f, ensure_ascii=False, indent=2, sort_keys=False)

    except Exception as e:
        print(f"Error processing {filepath}: {e}")

def main():
    if len(sys.argv) < 2:
        print("Usage: python format_json.py <file_or_directory>")
        sys.exit(1)

    target_path = sys.argv[1]

    if os.path.isfile(target_path):
        process_file(target_path)
    elif os.path.isdir(target_path):
        # Walk directory recursively
        for root, dirs, files in os.walk(target_path):
            for file in files:
                if file.lower().endswith('.json'):
                    full_path = os.path.join(root, file)
                    process_file(full_path)
    else:
        print(f"Error: Path not found: {target_path}")

if __name__ == "__main__":
    main()
