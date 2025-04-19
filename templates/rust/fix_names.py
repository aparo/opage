#!/usr/bin/env python3
import re
from pathlib import Path


REGEX_CHANGES = [(r"""@GET""", """@GetMapping""")]
TEXT_CHANGES = [
    ("""supportMultipleResponses""", """support_multiple_responses"""),
    ("""isDefault""", """is_default"""),
    ("""operationId""", """operation_id"""),
    ("""operation_idCamelCase""", """operation_id_camel_case"""),
    ("""returnType""", """return_type"""),
    ("""allParams""", """all_parameters"""),
    (""".isNullable """, """.is_nullable """),
    (""".isEnum """, """.is_enum """),
    (""".isArray """, """.is_array """),
    (""".uniqueItems """, """.unique_items """),
    (""".isEmpty """, """.is_empty """),
    (""".isInteger """, """.is_integer """),
    (""".isByteArray """, """.is_byte_array """),
    (""".enumName """, """.enum_name """),
    (""".isModel """, """.is_model """),
    (""".classFilename """, """.class_filename """),
    (""".oneOf""", """.one_of"""),
    (""".propertyBaseName""", """.property_base_name"""),
    ("""requiredVars""", """required_vars"""),
    ("""mappedModel""", """mapped_model"""),
    ("""modelName""", """model_name"""),
    ("""mappingName""", """mapping_name"""),
    ("""baseName""", """base_name"""),
    (""".composedSchemas""", """.composed_schemas"""),
    ("""avoidBoxedModels""", """avoid_boxed_models"""),
    (""" enumVar""", """ enum_var """),
    (""".allowableValues.enumVars""", """.allowable_values.enum_vars"""),
    (""".httpMethod """, """.http_method """),
    (""".queryParams """, """.query_parameters """),
    (""".pathParams """, """.path_parameters """),
    ("""operation.hasBodyParam """, """operation.has_body_parameters() """),
    ("""operation.bodyParams """, """operation.body_parameters """),
    ("""param.paramName""", """param.name"""),
    ("""param.baseName""", """param.base_name"""),
    ("""param.collectionFormat""", """param.collection_format"""),
    (""".isPrimitiveType """, """.is_primitive_type """),
    (""".dataType """, """.data_type """),
    (""".vendorExtensions.x_rust_has_byte_array """, """.rust_has_byte_array """),
    (
        """operation.vendorExtensions.x_group_parameters """,
        """operation.group_parameters """,
    ),
    (
        """operation.vendor_extensions.x_group_parameters """,
        """operation.group_parameters """,
    ),
    (""".baseType """, """.base_type """),
    (""".returnBaseType """, """.return_base_type """),
    (""".defaultValue """, """.default_value """),
    (""".authMethods """, """.auth_methods """),
    (""".useBonBuilder """, """.use_bon_builder """),
    (""".mediaType """, """.media_type """),
    ("""param.isArray """, """param.is_array """),
    ("""operation.hasAuthMethods """, """operation.has_auth_methods """),
    ("""auth.isApiKey """, """auth.is_api_key """),
    ("""auth.isOAuth """, """auth.is_oauth """),
    ("""auth.isBasic """, """auth.is_basic """),
    ("""auth.isCookie """, """auth.is_cookie """),
    ("""auth.isKeyInQuery """, """auth.is_key_in_query """),
    ("""auth.keyParamName """, """auth.key_param_name """),
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
