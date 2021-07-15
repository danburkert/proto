use std::collections::{BTreeMap, BTreeSet};
use std::fs::read_to_string;
use std::path::PathBuf;

use crate::{Config, Module};

#[derive(Debug)]
pub struct Mod {
    submodules: BTreeMap<String, Mod>,
    contents: Vec<Module>,
}

impl Mod {
    pub fn push(&mut self, module: &Module) {
        match module.as_slice() {
            [] => (),
            [name, left @ ..] => self.add(module.to_owned(), name, left),
        }
    }

    fn add(&mut self, module: Module, name: &str, left: &[String]) {
        let sub = self.submodules.entry(name.to_owned()).or_default();
        match left {
            [] => sub.contents.push(module),
            [name, left @ ..] => sub.add(module, name, left),
        }
    }
}

impl Default for Mod {
    fn default() -> Self {
        Self {
            submodules: BTreeMap::new(),
            contents: Vec::new(),
        }
    }
}

pub struct LibGenerator<'a> {
    config: &'a mut Config,
    depth: u8,
    buf: &'a mut String,
}

impl<'a> LibGenerator<'a> {
    pub fn generate_librs(config: &'a mut Config, mods: &Mod, buf: &'a mut String) {
        let mut generator = LibGenerator {
            config,
            depth: 0,
            buf,
        };

        if let Some(set) = generator.config.file_descriptor_set_path.clone() {
            generator.push_file_descriptor_set(set);
        }
        generator.push_mod(mods);
    }

    fn push_file_descriptor_set(&mut self, set_path: PathBuf) {
        self.buf.push_str("include!(\"");
        if self.config.manifest_tpl.is_some() {
            self.buf.push_str("../gen/");
        }
        self.buf.push_str(set_path.to_str().unwrap());
        self.buf.push_str(".rs\");\n");
    }

    fn push_mod(&mut self, mods: &Mod) {
        for (name, mods) in &mods.submodules {
            self.push_indent();
            self.buf.push_str("pub mod ");
            self.buf.push_str(name);
            self.buf.push_str(" {\n");
            self.depth += 1;
            self.push_mod(&mods);
            self.depth -= 1;
            self.push_indent();
            self.buf.push_str("}\n");
        }

        for package in mods.contents.iter().map(|content| content.join(".")) {
            if self.config.manifest_tpl.is_some() {
                let feature = package.replace("r#", "").replace(".", "_");
                self.push_indent();
                self.buf.push_str("#[cfg(feature = \"");
                self.buf.push_str(&feature);
                self.buf.push_str("\")]\n");
            }

            self.push_indent();
            self.buf.push_str("include!(\"");
            if self.config.manifest_tpl.is_some() {
                self.buf.push_str("../gen/");
            }
            self.buf.push_str(&package);
            self.buf.push_str(".rs\");\n");
        }
    }

    fn push_indent(&mut self) {
        for _ in 0..self.depth {
            self.buf.push_str("    ");
        }
    }

    pub fn generate_manifest(
        config: &'a mut Config,
        template: PathBuf,
        deps: BTreeMap<String, BTreeSet<String>>,
        buf: &mut String,
    ) {
        let mut generator = LibGenerator {
            config,
            depth: 0,
            buf,
        };
        generator.push_manifest(template, deps);
    }

    fn push_manifest(&mut self, template: PathBuf, deps: BTreeMap<String, BTreeSet<String>>) {
        let template = read_to_string(template).unwrap();
        let mut buf = String::new();
        for (feat, deps) in deps {
            buf.push('"');
            buf.push_str(&feat);
            buf.push_str("\" = [");
            let deps: Vec<_> = deps.iter().map(|dep| format!("\"{}\"", dep)).collect();
            buf.push_str(&deps.join(", "));
            buf.push_str("]\n");
        }
        self.buf.push_str(&template.replace("{{ features }}", &buf));
    }
}
