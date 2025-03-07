use askama::Template;

#[derive(Template)]
#[template(path = "rust/enum.j2")]
pub struct RustEnumTemplate<'a> {
    pub name: &'a str,
    pub description: &'a str,
    pub derivations: Vec<&'a str>,
    pub variants: Vec<String>,
}

#[derive(Template)]
#[template(path = "rust/type.j2")]
pub struct RustTypeTemplate<'a> {
    pub name: &'a str,
    pub value: &'a str,
    pub description: &'a str,
}
