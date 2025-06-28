# This is used to fix the typesense OpenAPI Spec because they use object types in query parameters which is not supported by our generator.

import yaml
import argparse


def flatten_query_parameters(input_file, output_file):
    with open(input_file, "r") as file:
        spec = yaml.safe_load(file)

    paths = spec.get("paths", {})
    # Iterate over the paths
    for path, path_item in paths.items():
        # Iterate over the operations (GET, POST, etc.)
        for method, operation in list(path_item.items()):
            if (
                path
                in [
                    "_/search",
                    "/_search/template",
                    "/_search_shards",
                    "/{index}/_search",
                    "/{index}/_search_shards",
                    "/{index}/_search/template",
                ]
                and method == "get"
            ):
                # Skip the search operation
                paths[path].pop(method)
                continue
            if path in ["_/bulk"] and method == "put":
                # Skip the search operation
                paths[path].pop(method)
                continue
            if path == "/{index}/_search" and method == "post":
                # Skip the search operation
                paths[path][method]["operationId"] = "search_with_index"
                continue
            if path == "/{index}/_search_shards" and method == "post":
                # Skip the search operation
                paths[path][method]["operationId"] = "search_shards_with_index"
                continue
            if path == "/{index}/_search/template" and method == "post":
                # Skip the search operation
                paths[path][method]["operationId"] = "search_template_with_index"
                continue
            operationId = paths[path][method]["operationId"]
            if "_superseded" in operationId:
                paths[path].pop(method)
                continue
            if operationId in ["bulk.2", "bulk.3"]:
                paths[path].pop(method)
                continue
            if operationId.startswith("cat.") and operationId.startswith(".1"):
                paths[path].pop(method)
                continue
            if operationId.startswith(".0"):
                paths[path][method]["operationId"] = operationId[:-2]
                continue

    # Save the modified spec
    with open(output_file, "w") as output_file:
        yaml.dump(spec, output_file)


def cook_operation(path: str, operationId: str) -> str:
    if operationId.startswith(".0"):
        operationId = operationId[:-2]

    return operationId


if __name__ == "__main__":
    # parser = argparse.ArgumentParser(
    #     description="Flatten object-type query parameters in OpenAPI spec."
    # )
    # parser.add_argument(
    #     "input_file", help="Path to the input OpenAPI spec file (YAML format)."
    # )
    # parser.add_argument(
    #     "output_file", help="Path to save the flattened OpenAPI spec file."
    # )

    # args = parser.parse_args()

    # flatten_query_parameters(args.input_file, args.output_file)
    flatten_query_parameters(
        "tests/projects/opensearch/spec.openapi.yaml",
        "tests/projects/opensearch/spec.openapi.fixed.yaml",
    )
