import os
import argparse
import re
from markdownify import markdownify as md


def filter_by_text(text):
    filter_words = ["сутта идентична", "Сутта идентична", "сутты в точности идентичны", "сутта в точности идентична", "сутта полностью идентична", "сутты идентичны", "эти шесть сутт идентичны", "Сутты идентичны", "полностью идентичны"]
    for word in filter_words:
        if word in text:
            return False

    return True

def escape_braces(text):
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

    return text

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
                    f.write(escape_braces(md_content))

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description='Convert HTML files to Markdown.')
    parser.add_argument('source', type=str, help='Source folder containing HTML files.')
    parser.add_argument('target', type=str, help='Target folder to save Markdown files.')
    args = parser.parse_args()

    html_to_md(args.source, args.target)
