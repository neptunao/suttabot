import os
import re
import json


QUOTE_LEVELS = (("„", "“"), ("«", "»"))


def _is_opening_quote(previous_non_space_char):
    if previous_non_space_char is None:
        return True

    return previous_non_space_char in "([{-—–/:;«„"


def replace_escaped_quotes(text):
    result = []
    in_string = False
    quote_stack = []
    previous_non_space_char = None
    index = 0

    while index < len(text):
        char = text[index]

        if not in_string:
            result.append(char)
            if char == '"':
                in_string = True
                quote_stack = []
                previous_non_space_char = None
            index += 1
            continue

        if char == '\\' and index + 1 < len(text):
            next_char = text[index + 1]

            if next_char == '"':
                if _is_opening_quote(previous_non_space_char):
                    level = len(quote_stack) % len(QUOTE_LEVELS)
                    quote_stack.append(level)
                    replacement = QUOTE_LEVELS[level][0]
                else:
                    level = quote_stack.pop() if quote_stack else 0
                    replacement = QUOTE_LEVELS[level][1]

                result.append(replacement)
                previous_non_space_char = replacement
                index += 2
                continue

            result.append(char)
            result.append(next_char)
            if not next_char.isspace():
                previous_non_space_char = next_char
            index += 2
            continue

        result.append(char)
        if char == '"':
            in_string = False
            quote_stack = []
            previous_non_space_char = None
        elif not char.isspace():
            previous_non_space_char = char
        index += 1

    return "".join(result)

def process_text(text):
    # Remove square brackets
    text = re.sub(r'\[(.*?)\]', r'\1', text)
    #replace ... with unicode ellipsis character
    text = text.replace('...', '…')

    # Replace escaped quotation marks inside JSON strings.
    text = replace_escaped_quotes(text)

    return text

def process_json_file(file_path):
    try:
        # Read the file
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()

        # Process the text
        processed_text = process_text(content)

        # Validate JSON
        try:
            json.loads(processed_text)
            is_valid_json = True
        except json.JSONDecodeError as e:
            is_valid_json = False
            print(f"Invalid JSON in {file_path}: {str(e)}")

        # Write back to the file only if JSON is valid
        if is_valid_json:
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write(processed_text)
            print(f"OK: {file_path}")
        else:
            print(f"NOT OK: {file_path} - JSON validation failed")

    except Exception as e:
        print(f"Error processing {file_path}: {str(e)}")

def process_directory(directory):
    # Walk through the directory recursively
    for root, _, files in os.walk(directory):
        for file in files:
            if file.endswith('.json'):
                file_path = os.path.join(root, file)
                process_json_file(file_path)

if __name__ == "__main__":
    # Get the directory path from command line argument
    import sys
    if len(sys.argv) != 2:
        print("Usage: python bilarify.py <directory_path>")
        sys.exit(1)

    directory_path = sys.argv[1]
    if not os.path.isdir(directory_path):
        print(f"Error: {directory_path} is not a valid directory")
        sys.exit(1)

    process_directory(directory_path)
