import os
import re
import json
from pathlib import Path

def process_text(text):
    # Remove square brackets
    text = re.sub(r'\[(.*?)\]', r'\1', text)

    # Replace quotation marks
    # First replace inner quotes (second level)
    text = re.sub(r'\\"([^"]*?)\\"', r'„\1  ', text)
    # Then replace outer quotes (first level)
    text = re.sub(r'\\"([^"]*?)\\"', r'«\1»', text)

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
