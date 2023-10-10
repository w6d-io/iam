use anyhow::{bail, Context, Result};
use tracing::{debug, info};

#[allow(unused_imports)]
use ory_kratos_client::{
    apis::{configuration::Configuration, identity_api::patch_identity},
    models::{Identity, JsonPatch},
};


use crate::permission::{Input, Mode};

async fn verify_type_path(
    _client: &Configuration,
    uuid: &str,
    payload: &Input,
) -> Result<Option<JsonPatch>> {
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
    debug!("identity: {:#?}", identity);
    let meta = match identity.metadata_admin {
        Some(meta) => meta,
        None => bail!("{uuid}: missing metadata_admin"),
    };
    if meta
        .pointer(&("/".to_owned() + &payload.perm_type as &str))
        .is_none()
    {
        info!(
            "{uuid}: {} do not exit, adding it to metadata",
            payload.perm_type
        );
        let path = "/metadata_admin".to_owned() + "/" + &payload.perm_type as &str;
        let patch = format!("{{\"op\" : \"add\", \"path\" : \"{path}\", \"value\" : {{}} }}");
        let json = serde_json::from_str::<JsonPatch>(&patch).context(format!("{uuid}:"))?;
        return Ok(Some(json));
    }
    Ok(None)
}

///make re request to change the identity coresponding to the user id
///with the given data and instruction,
pub async fn kratos_controler(
    _client: &Configuration,
    uuid: &str,
    payload: Input,
    op: &str,
) -> Result<()> {
    let mut patch_vec = Vec::new();
    if op != "remove" {
        if let Some(json_patch) = verify_type_path(_client, uuid, &payload).await? {
            patch_vec.push(json_patch);
        };
    }
    info!("{uuid}: Patching identity");
    let root = match payload.mode() {
        Mode::Admin => "metadata_admin",
        Mode::Public => "metadata_public",
        Mode::Trait => "trait",
    };

    let path =
        "/".to_owned() + root + "/" + &payload.perm_type as &str + "/" + &payload.resource as &str;
    let raw_patch = format!(
        "{{\"op\" : \"{op}\", \"path\" : \"{path}\", \"value\" : {}}}",
        payload.value
    );

    debug!("patch: {}", raw_patch);
    let patch = serde_json::from_str::<JsonPatch>(&raw_patch).context(format!("{uuid}:"))?;
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
        kratos_controler(&client, uuid, payload, "add")
            .await
            .unwrap();
    }
}
