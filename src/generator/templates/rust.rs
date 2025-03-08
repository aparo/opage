use askama::Template;
use clap::builder::Str;

#[derive(Template)]
#[template(path = "rust/enum.j2", escape = "none")]
pub struct RustEnumTemplate<'a> {
    pub imports: Vec<String>,
    pub derivations: Vec<&'a str>,
    pub description: &'a str,
    pub name: &'a str,
    pub variants: Vec<String>,
}

#[derive(Template)]
#[template(path = "rust/type.j2", escape = "none")]
pub struct RustTypeTemplate<'a> {
    pub name: &'a str,
    pub value: &'a str,
    pub description: &'a str,
}

#[derive(Debug, PartialOrd, PartialEq, Eq, Clone)]
pub struct Field {
    pub annotations: Vec<String>,
    pub description: String,
    pub modifier: String,
    pub name: String,
    pub typ: String,
}

impl Ord for Field {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(&other.name)
    }
}
#[derive(Template)]
#[template(path = "rust/struct.j2", escape = "none")]
pub struct RustStructTemplate<'a> {
    pub imports: Vec<String>,
    pub derivations: Vec<&'a str>,
    pub description: &'a str,
    pub name: &'a str,
    pub fields: Vec<Field>,
}
