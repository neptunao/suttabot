import json
import re

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

def json_to_markdown(json_data):
    markdown_lines = []
    previous_major = None

    for key, value in json_data.items():
        # Parse the key
        filecode, version = key.split(':')
        major_minor = version.split('.')

        # Handle major number ranges
        major_range = major_minor[0]
        if '-' in major_range:
            major_start, major_end = map(int, major_range.split('-'))
            major = (major_start, major_end)
        else:
            major = int(major_range)

        minor = int(major_minor[1])

        # Handle headings for major 0
        if major == 0:
            markdown_lines.append(f"**{value}**")
            markdown_lines.append("\n\n")
        else:
            # Add double new line if major number changes
            if previous_major is not None and previous_major != major:
                markdown_lines.append("\n\n")
            # Add space if major number is the same
            elif previous_major is not None:
                markdown_lines.append(" ")

            markdown_lines.append(value)

        previous_major = major

    return ''.join(markdown_lines)

# Example usage
json_data = {
  "thag8.3:0.1": "Стихи старших монахов 8.3 ",
  "thag8.3:0.2": "Восемь строф ",
  "thag8.3:0.3": "Глава первая ",
  "thag8.3:0.4": "Махапантхака Тхера ",
  "thag8.3:1.1": "Когда увидел я наставника, ",
  "thag8.3:1.2": "Свободного всецело от любого страха, ",
  "thag8.3:1.3": "Я чувства неотложности исполнился, ",
  "thag8.3:1.4": "Ведь встретил лучшего средь всех людей. ",
  "thag8.3:2.1": "Зреть Совершенного есть редкая удача, ",
  "thag8.3:2.2": "Как смертному её не упустить? ",
  "thag8.3:2.3": "Я выразил ему своё почтенье. ",
  "thag8.3:3.1": "Оставив позади сынов, жену, ",
  "thag8.3:3.2": "Отринув достаток и зерно, ",
  "thag8.3:3.3": "Я волосы обрил, ",
  "thag8.3:3.4": "Ушёл в скитанья. ",
  "thag8.3:4.1": "Блюдя монашества священные заветы, ",
  "thag8.3:4.2": "Хранящий двери чувств, ",
  "thag8.3:4.3": "Живу я в почитанье благородных, ",
  "thag8.3:4.4": "Необоримый. ",
  "thag8.3:5.1": "В решимости достичь конца пути, ",
  "thag8.3:5.2": "Что рождена в глубинах сердца, ",
  "thag8.3:5.3": "Что не даёт на месте усидеть, ",
  "thag8.3:5.4": "Покуда стрелы жажды я не выну из груди. ",
  "thag8.3:6.1": "Так проходила жизнь, ",
  "thag8.3:6.2": "Вы удивитесь рьяности такой! ",
  "thag8.3:6.3": "Три знания открылись для меня, ",
  "thag8.3:6.4": "Заветы Татхагаты мной исполнены всецело. ",
  "thag8.3:7.1": "Я вспомнил прошлые обители свои, ",
  "thag8.3:7.2": "Очищенным божественным оком; ",
  "thag8.3:7.3": "Я арахант, достойный подношений, ",
  "thag8.3:7.4": "Освобождённый от цепляний всех. ",
  "thag8.3:8.1": "Уж близится ночи конец, ",
  "thag8.3:8.2": "Рассвет забрезжил вдалеке; ",
  "thag8.3:8.3": "Вся жажда высохла во мне, ",
  "thag8.3:8.4": "Сидящем со скрещёнными ногами. "
}

markdown = escape_braces(json_to_markdown(json_data))
# save to file
with open('mn1.md', 'w') as f:
    f.write(markdown)
