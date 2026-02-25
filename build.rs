use std::fs;
use std::path::PathBuf;

use reqwest::Error;

fn main() {
    match download_anilist_schema() {
        Ok(_) => (),
        Err(err) => panic!("{}", err),
    }
}

fn download_anilist_schema() -> Result<(), Error> {
    let schema_dir = PathBuf::from("schema");
    let schema_path = schema_dir.join("anilist_schema.json");

    // Skip download if schema already exists (for offline builds)
    if schema_path.exists() {
        println!("cargo:rerun-if-changed={}", schema_path.display());
        return Ok(());
    }

    println!("cargo:warning=Downloading AniList GraphQL schema");

    // Create graphql directory if it doesn't exist
    fs::create_dir_all(&schema_dir).expect("Failed to create graphql directory");

    // GraphQL introspection query (standard format)
    let introspection_query = r#"
    query IntrospectionQuery {
      __schema {
        queryType { name }
        mutationType { name }
        subscriptionType { name }
        types {
          ...FullType
        }
        directives {
          name
          description
          locations
          args {
            ...InputValue
          }
        }
      }
    }

    fragment FullType on __Type {
      kind
      name
      description
      fields(includeDeprecated: true) {
        name
        description
        args {
          ...InputValue
        }
        type {
          ...TypeRef
        }
        isDeprecated
        deprecationReason
      }
      inputFields {
        ...InputValue
      }
      interfaces {
        ...TypeRef
      }
      enumValues(includeDeprecated: true) {
        name
        description
        isDeprecated
        deprecationReason
      }
      possibleTypes {
        ...TypeRef
      }
    }

    fragment InputValue on __InputValue {
      name
      description
      type { ...TypeRef }
      defaultValue
    }

    fragment TypeRef on __Type {
      kind
      name
      ofType {
        kind
        name
        ofType {
          kind
          name
          ofType {
            kind
            name
            ofType {
              kind
              name
              ofType {
                kind
                name
                ofType {
                  kind
                  name
                  ofType {
                    kind
                    name
                  }
                }
              }
            }
          }
        }
      }
    }
    "#;

    // Make HTTP request using reqwest blocking client
    let client = reqwest::blocking::Client::new();
    let response = client
        .post("https://graphql.anilist.co")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&serde_json::json!({
            "query": introspection_query
        }))
        .send()?;

    let json = response.json::<serde_json::Value>()?;

    let schema_json = serde_json::to_string_pretty(&json).expect("Failed to serialize schema");
    fs::write(&schema_path, schema_json).expect("Failed to write schema file");

    println!("cargo:rerun-if-changed={}", schema_path.display());

    Ok(())
}
