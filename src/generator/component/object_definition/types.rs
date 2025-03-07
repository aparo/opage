use crate::generator::templates::rust::RustEnumTemplate;
use crate::utils::{
    config::Config,
    name_mapping::{extract_rust_name, fix_rust_description},
};
use askama::Template;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub struct ModuleInfo {
    pub name: String,
    pub path: String,
}

impl ModuleInfo {
    pub fn new(path: &str, name: &str) -> Self {
        let mut final_name = name.to_string();
        let mut final_path = path.to_string();
        if final_name.contains("::") {
            let parts: Vec<&str> = name.split("::").collect();
            final_name = parts[parts.len() - 1].to_owned();
            for sep in parts[..parts.len() - 1].iter() {
                if final_path.contains(format!("::{}", sep).as_str()) {
                    continue;
                }
                if !final_path.is_empty() {
                    final_path.push_str("::");
                }
                final_path.push_str(sep);
            }
        }

        ModuleInfo {
            name: final_name,
            path: final_path,
        }
    }

    pub fn to_use(&self) -> String {
        if self.path.is_empty() {
            return format!("use {};", self.name);
        }
        format!("use {}::{};", self.path, self.name)
    }
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

// pub type ObjectDatabase = HashMap<String, ObjectDefinition>;
#[derive(Clone, Debug)]
pub struct ObjectDatabase {
    objects: HashMap<String, ObjectDefinition>,
}

impl ObjectDatabase {
    pub fn new() -> Self {
        ObjectDatabase {
            objects: HashMap::new(),
        }
    }

    pub fn keys(&self) -> std::collections::hash_map::Keys<String, ObjectDefinition> {
        self.objects.keys()
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.objects.contains_key(key)
    }

    pub fn insert(&mut self, key: &str, object: ObjectDefinition) {
        self.objects.insert(key.to_owned(), object);
    }

    pub fn get(&self, id: &str) -> Option<&ObjectDefinition> {
        self.objects.get(id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut ObjectDefinition> {
        self.objects.get_mut(id)
    }

    pub fn remove(&mut self, id: &str) -> Option<ObjectDefinition> {
        self.objects.remove(id)
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<String, ObjectDefinition> {
        self.objects.iter()
    }

    pub fn len(&self) -> usize {
        self.objects.len()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnumDefinition {
    pub name: String,
    // pub namespace: String,
    pub used_modules: Vec<ModuleInfo>,
    pub values: HashMap<String, EnumValue>,
    pub description: Option<String>,
}

impl EnumDefinition {
    // pub fn id(&self) -> String {
    //     format!("{}::{}", self.namespace, self.name)
    // }

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
        // let mut definition_str = String::new();
        let description =
            fix_rust_description("", &self.description.as_ref().map_or("", |d| d.as_str()));
        let variants = self
            .values
            .iter()
            .map(|(_, enum_value)| {
                format!(
                    "{}({})",
                    extract_rust_name(&enum_value.name),
                    extract_rust_name(&enum_value.value_type.name)
                )
            })
            .collect();

        let mut derivations = vec!["Debug", "Clone", "PartialEq"];
        if serializable {
            derivations.push("Serialize");
            derivations.push("Deserialize");
        }

        let template = RustEnumTemplate {
            name: extract_rust_name(&self.name).as_str(),
            description: description.as_str(),
            derivations,
            variants: variants,
        }
        .render()
        .unwrap();
        template
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StructDefinition {
    pub package: String,
    pub name: String,
    pub used_modules: Vec<ModuleInfo>,
    pub properties: HashMap<String, PropertyDefinition>,
    pub local_objects: HashMap<String, Box<ObjectDefinition>>,
    pub description: Option<String>,
}

impl StructDefinition {
    pub fn id(&self) -> String {
        format!("{}::{}", self.package, self.name)
    }

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
            definition_str.push_str(fix_rust_description("", def).as_str());
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
                definition_str.push_str(fix_rust_description("  ", def).as_str());
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
