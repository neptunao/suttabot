import os
import argparse
import re
from markdownify import markdownify as md


def filter_by_text(text):
    filter_words = [
        "сутта идентична",
        "Сутта идентична",
        "сутты в точности идентичны",
        "сутта в точности идентична",
        "сутта полностью идентична",
        "сутты идентичны",
        "эти шесть сутт идентичны",
        "Сутты идентичны",
        "полностью идентичны",
        "в точности аналогичн"
    ]
    for word in filter_words:
        if word in text:
            return False

    return True

def to_telegram_markdown(text):
    # Regex pattern for markdown links
    markdown_link_pattern = r'\[.*?\]\(.*?\)'

    # Find all markdown links
    markdown_links = re.findall(markdown_link_pattern, text)

    # Replace braces with escaped braces
    text = text.replace('{', '\\{').replace('}', '\\}').replace('[', '\\[').replace(']', '\\]').replace('(', '\\(').replace(')', '\\)')

    # Replace escaped braces in markdown links with unescaped braces
    for link in markdown_links:
        escaped_link = link.replace('[', '\\[').replace(']', '\\]').replace('(', '\\(').replace(')', '\\)')
        text = text.replace(escaped_link, link)

    text = (text.replace('`', '\\`')
                .replace('#', '\\#')
                .replace('.', '\\.')
                .replace('!', '\\!')
                .replace('=', '\\=')
                .replace('-', '\\-'))

    # Fix unclosed markdown formatting
    lines = text.split('\n')
    for i, line in enumerate(lines):
        # Fix unclosed bold/italic text (asterisks)
        if line.count('*') % 2 != 0:
            # Check if line starts with an asterisk but doesn't end with one
            if line.startswith('*') and not line.endswith('*'):
                lines[i] = line + '*'
            # Check if line ends with an asterisk but doesn't start with one
            elif line.endswith('*') and not line.startswith('*'):
                lines[i] = '*' + line
            # Otherwise, just add an asterisk at the end
            else:
                lines[i] = line + '*'

        # Fix unclosed italic text (single underscore)
        if line.count('_') % 2 != 0:
            # Check if line starts with an underscore but doesn't end with one
            if line.startswith('_') and not line.endswith('_'):
                lines[i] = line + '_'
            # Check if line ends with an underscore but doesn't start with one
            elif line.endswith('_') and not line.startswith('_'):
                lines[i] = '_' + line
            # Otherwise, just add an underscore at the end
            else:
                lines[i] = line + '_'

        # Fix unclosed bold text (double underscore)
        # Count pairs of double underscores
        double_underscore_count = line.count('__')
        if double_underscore_count % 2 != 0:
            # If there's an odd number of double underscores, add one more at the end
            lines[i] = line + '__'

    return '\n'.join(lines)

def html_to_md(source_folder, target_folder):
    os.makedirs(target_folder, exist_ok=True)

    for dirpath, dirnames, filenames in os.walk(source_folder):
        for filename in filenames:
            if filename.endswith('.html') or filename.endswith('.htm'):
                source_file = os.path.join(dirpath, filename)
                target_file = os.path.join(target_folder, filename.rsplit('.', 1)[0] + '.md')

                with open(source_file, 'r') as f:
                    html_content = f.read()

                md_content = md(html_content)

                if not filter_by_text(md_content):
                    continue

                with open(target_file, 'w') as f:
                    f.write(to_telegram_markdown(md_content))

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description='Convert HTML files to Markdown.')
    parser.add_argument('source', type=str, help='Source folder containing HTML files.')
    parser.add_argument('target', type=str, help='Target folder to save Markdown files.')
    args = parser.parse_args()

    html_to_md(args.source, args.target)
