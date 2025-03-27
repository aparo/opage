#!/usr/bin/env python3
import re
from pathlib import Path


REGEX_CHANGES = [(r"""@GET""", """@GetMapping""")]
TEXT_CHANGES = [
    ("""supportMultipleResponses""", """support_multiple_responses"""),
    ("""isDefault""", """is_default"""),
    ("""operationId""", """operation_id"""),
    ("""returnType""", """return_type"""),
    ("""allParams""", """all_parameters"""),
    (""".isNullable """, """.is_nullable """),
    (""".httpMethod """, """.http_method """),
    ("""param.paramName""", """param.name"""),
    (""".isPrimitiveType """, """.is_primitive_type """),
    (""".dataType """, """.data_type """),
    (""".baseType """, """.base_type """),
    (""".returnBaseType """, """.return_base_type """),
    (""".defaultValue """, """.default_value """),
    (""".authMethods """, """.auth_methods """),
    (""".useBonBuilder """, """.use_bon_builder """),
    (""".mediaType """, """.media_type """),
    (""" invokerPackage """, """ invoker_package """),
    ("""{{ basePath }}""", """{{ base_path }}"""),
    ("""% if not """, """% if !"""),
]

extensions = set(".j2 .jinga2".split())

skipped = set()

for filename in Path.cwd().rglob("*"):
    if (
        ".git" in str(filename)
        or ".husky" in str(filename)
        or ".venv" in str(filename)
        or "node_modules" in str(filename)
    ):
        continue
    if not filename.is_file():
        continue
    if filename.suffix not in extensions:
        skipped.add(filename.suffix)
        continue

    print(f"Checking {filename}")
    original = filename.read_text()
    content = original

    for old_value, new_value in TEXT_CHANGES:
        content = content.replace(old_value, new_value)
    for rx, new_value in REGEX_CHANGES:
        matches = re.finditer(rx, content, re.MULTILINE)
        for match in matches:
            item = match.group()
            groups = match.groups()
            rep = new_value
            if len(groups) > 0:
                for pos, g in enumerate(groups):
                    rep = rep.replace("$" + str(pos + 1), g)
            content = content.replace(item, rep)

    if original != content:
        print(f"Updating {filename}")
        filename.write_text(content)

from pprint import pprint

pprint(skipped)
