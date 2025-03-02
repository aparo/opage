use std::collections::HashMap;

use crate::utils::config::Config;

#[derive(Clone, Debug, PartialEq)]
pub struct ModuleInfo {
    pub name: String,
    pub path: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TypeDefinition {
    pub name: String,
    pub module: Option<ModuleInfo>,
    pub description: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PropertyDefinition {
    pub name: String,
    pub real_name: String,
    pub type_name: String,
    pub module: Option<ModuleInfo>,
    pub required: bool,
    pub description: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ObjectDefinition {
    Struct(StructDefinition),
    Enum(EnumDefinition),
    Primitive(PrimitiveDefinition),
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnumValue {
    pub name: String,
    pub value_type: TypeDefinition,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnumDefinition {
    pub name: String,
    pub used_modules: Vec<ModuleInfo>,
    pub values: HashMap<String, EnumValue>,
    pub description: Option<String>,
}

pub type ObjectDatabase = HashMap<String, ObjectDefinition>;

impl EnumDefinition {
    pub fn get_required_modules(&self) -> Vec<&ModuleInfo> {
        let mut required_modules = self.used_modules.iter().collect::<Vec<&ModuleInfo>>();
        required_modules.append(
            &mut self
                .values
                .iter()
                .filter_map(|(_, enum_value)| enum_value.value_type.module.as_ref())
                .collect::<Vec<&ModuleInfo>>(),
        );
        required_modules
    }

    pub fn to_string(&self, serializable: bool) -> String {
        let mut definition_str = String::new();

        if let Some(desc) = &self.description {
            definition_str.push_str(format_description("", desc).as_str());
        }
        if serializable {
            definition_str.push_str("#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]\n")
        };
        definition_str.push_str(format!("pub enum {} {{\n", self.name).as_str());

        for (_, enum_value) in &self.values {
            definition_str.push_str(
                format!("  {}({}),\n", enum_value.name, enum_value.value_type.name).as_str(),
            );
        }

        definition_str.push_str("}");
        definition_str
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StructDefinition {
    pub used_modules: Vec<ModuleInfo>,
    pub name: String,
    pub properties: HashMap<String, PropertyDefinition>,
    pub local_objects: HashMap<String, Box<ObjectDefinition>>,
    pub description: Option<String>,
}

impl StructDefinition {
    pub fn get_required_modules(&self) -> Vec<&ModuleInfo> {
        let mut required_modules = self.used_modules.iter().collect::<Vec<&ModuleInfo>>();
        required_modules.append(
            &mut self
                .properties
                .iter()
                .filter_map(|(_, property)| property.module.as_ref())
                .collect::<Vec<&ModuleInfo>>(),
        );
        required_modules
    }

    pub fn to_string(&self, serializable: bool, config: &Config) -> String {
        let mut definition_str = String::new();
        if let Some(def) = &self.description {
            definition_str.push_str(format_description("", def).as_str());
        }

        if serializable {
            definition_str.push_str("#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]\n");
        };
        definition_str.push_str(format!("pub struct {} {{\n", self.name).as_str());

        for (_, property) in &self.properties {
            let mut serde_parts = vec![];
            if serializable
                && (property.name != property.real_name || is_private_name(&property.real_name))
            {
                serde_parts.push(format!("alias = \"{}\"", property.real_name));
            }

            if let Some(def) = &property.description {
                definition_str.push_str(format_description("  ", def).as_str());
            }

            if property.type_name.starts_with("Vec<") {
                serde_parts.push("default".to_string());
                serde_parts.push("skip_serializing_if = \"Vec::is_empty\"".to_string());
            } else if property.type_name.starts_with("Map<") {
                serde_parts.push("default".to_string());
                serde_parts.push("skip_serializing_if = \"Map::is_empty\"".to_string());
            } else if !property.required && serializable {
                if config.serde_skip_null {
                    serde_parts.push("default".to_string());
                    serde_parts.push("skip_serializing_if = \"Option::is_none\"".to_string());
                } else {
                    serde_parts.push("default".to_string());
                }
            }

            if property.required
                || property.type_name.starts_with("Vec<")
                || property.type_name.starts_with("Map<")
            {
                if !serde_parts.is_empty() {
                    definition_str
                        .push_str(format!("  #[serde({})]\n", serde_parts.join(", ")).as_str());
                }
                definition_str.push_str(
                    format!(
                        "  pub {}: {},\n",
                        fix_private_name(&property.name),
                        property.type_name
                    )
                    .as_str(),
                );
            } else {
                if serializable {
                    definition_str
                        .push_str(format!("  #[serde({})]\n", serde_parts.join(", ")).as_str());
                }
                definition_str.push_str(
                    format!(
                        "  pub {}: Option<{}>,\n",
                        fix_private_name(&property.name),
                        property.type_name
                    )
                    .as_str(),
                );
            }
        }

        definition_str.push('}');
        definition_str
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PrimitiveDefinition {
    pub name: String,
    pub primitive_type: TypeDefinition,
    pub description: Option<String>,
}

fn format_description(ident: &str, description: &str) -> String {
    description
        .lines()
        .map(|line| format!("{}/// {}\n", ident, line))
        .collect::<String>()
}

fn is_private_name(name: &str) -> bool {
    name.eq_ignore_ascii_case("type") || name.starts_with("r#")
}

fn fix_private_name(name: &str) -> String {
    if name.eq_ignore_ascii_case("type") {
        "r#type".to_string()
    } else {
        name.to_string()
    }
}
