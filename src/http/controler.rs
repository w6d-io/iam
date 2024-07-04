use anyhow::{Context, Result};
use serde_json::{json, Value};
use tracing::{debug, info};

#[allow(unused_imports)]
use ory_kratos_client::{
    apis::{configuration::Configuration, identity_api::patch_identity},
    models::{Identity, JsonPatch},
};

use crate::permission::{Input, Mode};

///This function create an empty map exist at asked path.
fn patch_empty_meta(root: &str, patch_vec: &mut Vec<JsonPatch>, uuid: &str) -> Result<()> {
    let path = "/".to_owned() + root;
    let patch = json!({
        "op": "add",
        "path": path,
        "value" : {}
    });
    let json = serde_json::from_value::<JsonPatch>(patch).context(format!("{uuid}:"))?;
    patch_vec.push(json);
    Ok(())
}

///This function make sure that an empty map of the type being patch exist if not it add an empty
///one.
async fn verify_type_path(
    _client: &Configuration,
    uuid: &str,
    payload: &Input,
) -> Result<Option<Vec<JsonPatch>>> {
    let mut patch_vec = Vec::new();
    #[cfg(not(test))]
    let identity =
        ory_kratos_client::apis::identity_api::get_identity(_client, &payload.id, None).await?;
    #[cfg(test)]
    let identity = {
        let mut identity = Identity::new(
            "test".to_owned(),
            "test".to_owned(),
            "test".to_owned(),
            None,
        );
        identity.metadata_admin = Some(serde_json::Value::String("test".to_owned()));
        identity
    };
    let (root, meta) = match payload.mode() {
        Mode::Admin => {
            let root = "metadata_admin";
            let meta = match &identity.metadata_admin {
                Some(meta) => meta,
                None => {
                    patch_empty_meta(root, &mut patch_vec, uuid)?;
                    &Value::Null
                }
            };
            (root, meta)
        }
        Mode::Public => {
            let root = "metadata_public";
            let meta = match &identity.metadata_public {
                Some(meta) => meta,
                None => {
                    patch_empty_meta(root, &mut patch_vec, uuid)?;
                    &Value::Null
                }
            };
            (root, meta)
        }
        Mode::Trait => {
            let root = "traits";
            let meta = match &identity.traits {
                Some(meta) => meta,
                None => {
                    patch_empty_meta(root, &mut patch_vec, uuid)?;
                    &Value::Null
                }
            };
            (root, meta)
        }
    };
    debug!("identity: {:#?}", identity);
    let pointer = meta.pointer(&("/".to_owned() + &payload.perm_type as &str));
    if pointer.is_none() {
        info!(
            "{uuid}: {} do not exit, adding it to metadata",
            payload.perm_type
        );
        let path = "/".to_owned() + root + "/" + &payload.perm_type as &str;
        let value = serde_json::from_str::<Value>(&payload.value)?;
        let patch = if value.is_null() || payload.resource == "-" {
            json!({"op" : "add", "path" : path, "value" : [] })
        } else {
            json!({"op" : "add", "path" : path, "value" : {} })
        };
        let json = serde_json::from_value::<JsonPatch>(patch).context(format!("{uuid}:"))?;
        patch_vec.push(json);
        return Ok(Some(patch_vec));
    }
    Ok(None)
}

///Make request to change the identity coresponding to the user id (uuid)
///with the given data (payload) and instruction (op).
pub async fn kratos(_client: &Configuration, uuid: &str, payload: Input, op: &str) -> Result<()> {
    let mut patch_vec = Vec::new();
    if op != "remove" {
        if let Some(mut json_patch) = verify_type_path(_client, uuid, &payload).await? {
            patch_vec.append(&mut json_patch);
        };
    }
    info!("{uuid}: Patching identity");
    let root = match payload.mode() {
        Mode::Admin => "metadata_admin",
        Mode::Public => "metadata_public",
        Mode::Trait => "trait",
    };
    let mut value = serde_json::from_str::<Value>(&payload.value)?;
    let path = match value {
        Value::Null => {
            value = Value::String(payload.resource);
            "/".to_owned() + root + "/" + &payload.perm_type as &str + "/-"
        }
        _ => {
            "/".to_owned()
                + root
                + "/"
                + &payload.perm_type as &str
                + "/"
                + &payload.resource as &str
        }
    };
    let raw_patch = json!({
        "op": op,
        "path": path,
        "value": value}
    );

    debug!("patch: {}", raw_patch);
    let patch = serde_json::from_value::<JsonPatch>(raw_patch).context(format!("{uuid}:"))?;
    patch_vec.push(patch);
    debug!("vec patch: {:?}", patch_vec);
    #[cfg(not(test))]
    patch_identity(_client, &payload.id, Some(patch_vec))
        .await
        .context(format!("{uuid}:"))?;
    info!("{uuid}: Identity patching sucessfull");
    Ok(())
}

#[cfg(test)]
mod test_controler {
    use super::*;

    #[tokio::test]
    async fn test_kratos_controler() {
        let client = Configuration::default();
        let uuid = "test";
        let payload = Input {
            id: "1".to_owned(),
            perm_type: "test".to_owned(),
            resource: "resource".to_owned(),
            value: "\"testting\"".to_owned(),
            mode: 0,
        };
        kratos(&client, uuid, payload, "add").await.unwrap();
    }
}
