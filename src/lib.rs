//! Openapi provides structures and support for serializing and deserializing [openapi](https://github.com/OAI/OpenAPI-Specification) specifications
//!
//! # Examples
//!
//! Typical use deserialing an existing to a persisted spec to rust form or
//! visa versa
//!
//! The hyper client should be configured with tls.
//!
//! ```no_run
//! extern crate openapi;
//!
//! fn main() {
//!   match openapi::from_path("path/to/openapi.yaml") {
//!     Ok(spec) => println!("spec: {:?}", spec),
//!     Err(err) => println!("error: {}", err)
//!   }
//! }
//! ```
//!
//! # Errors
//!
//! Operations typically result in a `openapi::Result` Type which is an alias
//! for Rust's
//! built-in Result with the Err Type fixed to the
//! [openapi::errors::Error](errors/struct.Error.html) enum type. These are
//! provided
//! using [error_chain](https://github.com/brson/error-chain) crate so their
//! shape and behavior should be consistent and familiar to existing
//! error_chain users.
//!

#[macro_use]
extern crate serde_derive;

use std::{fs::File, io::Read, path::Path, result::Result as StdResult};

mod error;
pub use error::Error;

mod components;
mod contact;
mod encoding;
mod example;
mod external_doc;
mod flows;
mod header;
mod info;
mod license;
mod link;
mod media_type;
mod media_type_examples;
mod operation;
mod parameter;
mod path_item;
mod request_body;
mod response;
mod schema;
mod security_scheme;
mod server;
mod spec;
mod tag;
mod url;

pub use self::url::*;
pub use components::*;
pub use contact::*;
pub use encoding::*;
pub use example::*;
pub use external_doc::*;
pub use flows::*;
pub use header::*;
pub use info::*;
pub use license::*;
pub use link::*;
pub use media_type::*;
pub use media_type_examples::*;
pub use operation::*;
pub use parameter::*;
pub use path_item::*;
pub use request_body::*;
pub use response::*;
pub use schema::*;
pub use security_scheme::*;
pub use server::*;
pub use spec::*;
pub use tag::*;

pub type Result<T> = StdResult<T, Error>;

/// Version 3.0.1 of the OpenApi specification.
///
/// Refer to the official
/// [specification](https://github.com/OAI/OpenAPI-Specification/blob/0dd79f6/versions/3.0.1.md)
/// for more information.
pub type OpenApiV3 = Spec;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(untagged)]
pub enum ObjectOrReference<T> {
    Ref {
        #[serde(rename = "$ref")]
        ref_path: String,
    },
    Object(T),
}

impl<T> ObjectOrReference<T>
where
    T: FromRef,
{
    pub fn resolve(&self, spec: &Spec) -> Option<T> {
        match self {
            Self::Object(component) => Some(component.clone()),
            Self::Ref { ref_path } => T::from_ref(&spec, &ref_path),
        }
    }
}

pub struct RefPath {
    kind: String,
    name: String,
}

impl RefPath {
    // TODO: impl FromStr
    pub fn from_str<'a>(path: &'a str) -> Self {
        // assume path starts with #/
        let path = &path[13..];
        println!("path = {:?}", &path);

        let parts = &path.split('/').collect::<Vec<_>>();

        Self {
            kind: parts[0].to_owned(),
            name: parts[1].to_owned(),
        }
    }
}

pub trait FromRef: Clone {
    fn from_ref(spec: &Spec, path: &str) -> Option<Self>
    where
        Self: Sized;
}

/// deserialize an open api spec from a path
pub fn from_path<P>(path: P) -> Result<OpenApiV3>
where
    P: AsRef<Path>,
{
    from_reader(File::open(path)?)
}

/// deserialize an open api spec from type which implements Read
pub fn from_reader<R>(read: R) -> Result<OpenApiV3>
where
    R: Read,
{
    Ok(serde_yaml::from_reader::<R, OpenApiV3>(read)?)
}

/// serialize to a yaml string
pub fn to_yaml(spec: &OpenApiV3) -> Result<String> { Ok(serde_yaml::to_string(spec)?) }

/// serialize to a json string
pub fn to_json(spec: &OpenApiV3) -> Result<String> { Ok(serde_json::to_string_pretty(spec)?) }

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::{
        fs::{self, read_to_string, File},
        io::Write,
    };

    /// Helper function to write string to file.
    fn write_to_file<P>(path: P, filename: &str, data: &str)
    where
        P: AsRef<Path> + std::fmt::Debug,
    {
        println!("    Saving string to {:?}...", path);
        std::fs::create_dir_all(&path).unwrap();
        let full_filename = path.as_ref().to_path_buf().join(filename);
        let mut f = File::create(&full_filename).unwrap();
        f.write_all(data.as_bytes()).unwrap();
    }

    /// Convert a YAML `&str` to a JSON `String`.
    fn convert_yaml_str_to_json(yaml_str: &str) -> String {
        let yaml: serde_yaml::Value = serde_yaml::from_str(yaml_str).unwrap();
        let json: serde_json::Value = serde_yaml::from_value(yaml).unwrap();
        serde_json::to_string_pretty(&json).unwrap()
    }

    /// Deserialize and re-serialize the input file to a JSON string through two different
    /// paths, comparing the result.
    /// 1. File -> `String` -> `serde_yaml::Value` -> `serde_json::Value` -> `String`
    /// 2. File -> `Spec` -> `serde_json::Value` -> `String`
    /// Both conversion of `serde_json::Value` -> `String` are done
    /// using `serde_json::to_string_pretty`.
    /// Since the first conversion is independant of the current crate (and only
    /// uses serde's json and yaml support), no information should be lost in the final
    /// JSON string. The second conversion goes through our `OpenApi`, so the final JSON
    /// string is a representation of _our_ implementation.
    /// By comparing those two JSON conversions, we can validate our implementation.
    fn compare_spec_through_json(
        input_file: &Path,
        save_path_base: &Path,
    ) -> (String, String, String) {
        // First conversion:
        //     File -> `String` -> `serde_yaml::Value` -> `serde_json::Value` -> `String`

        // Read the original file to string
        let spec_yaml_str = read_to_string(&input_file)
            .unwrap_or_else(|e| panic!("failed to read contents of {:?}: {}", input_file, e));
        // Convert YAML string to JSON string
        let spec_json_str = convert_yaml_str_to_json(&spec_yaml_str);

        // Second conversion:
        //     File -> `Spec` -> `serde_json::Value` -> `String`

        // Parse the input file
        let parsed_spec = from_path(&input_file).unwrap();
        // Convert to serde_json::Value
        let parsed_spec_json = serde_json::to_value(parsed_spec).unwrap();
        // Convert to a JSON string
        let parsed_spec_json_str: String = serde_json::to_string_pretty(&parsed_spec_json).unwrap();

        // Save JSON strings to file
        let api_filename = input_file
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .replace(".yaml", ".json");

        let mut save_path = save_path_base.to_path_buf();
        save_path.push("yaml_to_json");
        write_to_file(&save_path, &api_filename, &spec_json_str);

        let mut save_path = save_path_base.to_path_buf();
        save_path.push("yaml_to_spec_to_json");
        write_to_file(&save_path, &api_filename, &parsed_spec_json_str);

        // Return the JSON filename and the two JSON strings
        (api_filename, parsed_spec_json_str, spec_json_str)
    }

    // Just tests if the deserialization does not blow up. But does not test correctness
    #[test]
    fn can_deserialize() {
        for entry in fs::read_dir("data/v2").unwrap() {
            let path = entry.unwrap().path();
            // cargo test -- --nocapture to see this message
            println!("Testing if {:?} is deserializable", path);
            from_path(path).unwrap();
        }
    }

    #[test]
    fn can_deserialize_and_reserialize_v2() {
        let save_path_base: std::path::PathBuf =
            ["target", "tests", "can_deserialize_and_reserialize_v2"]
                .iter()
                .collect();

        for entry in fs::read_dir("data/v2").unwrap() {
            let path = entry.unwrap().path();

            println!("Testing if {:?} is deserializable", path);

            let (api_filename, parsed_spec_json_str, spec_json_str) =
                compare_spec_through_json(&path, &save_path_base);

            assert_eq!(
                parsed_spec_json_str.lines().collect::<Vec<_>>(),
                spec_json_str.lines().collect::<Vec<_>>(),
                "contents did not match for api {}",
                api_filename
            );
        }
    }

    #[test]
    fn can_deserialize_and_reserialize_v3() {
        let save_path_base: std::path::PathBuf =
            ["target", "tests", "can_deserialize_and_reserialize_v3"]
                .iter()
                .collect();

        for entry in fs::read_dir("data/v3.0").unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();

            println!("Testing if {:?} is deserializable", path);

            let (api_filename, parsed_spec_json_str, spec_json_str) =
                compare_spec_through_json(&path, &save_path_base);

            assert_eq!(
                parsed_spec_json_str.lines().collect::<Vec<_>>(),
                spec_json_str.lines().collect::<Vec<_>>(),
                "contents did not match for api {}",
                api_filename
            );
        }
    }
}
