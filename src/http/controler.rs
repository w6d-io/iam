use anyhow::{Context, Ok, Result};

#[allow(unused_imports)]
use ory_kratos_client::{
    apis::{configuration::Configuration, identity_api::patch_identity},
    models::JsonPatch,
};

use crate::permission::Input;

///make re request to change the identity coresponding to the user id
///with the given data and instruction,
pub async fn kratos_controler(
    _client: &Configuration,
    uuid: &str,
    payload: Input,
    op: &str,
) -> Result<()> {
    let path =
        "metadata/type/".to_owned() + &payload.perm_type as &str + "/" + &payload.resource as &str;
    let patch = format!(
        "{{\"op\" : \"{op}\", \"path\" : \"{path}\", \"value\" : \"{}\"}}",
        payload.role
    );
    let _patch = serde_json::from_str::<JsonPatch>(&patch).context(format!("{uuid}:"))?;
    #[cfg(not(test))]
    patch_identity(_client, &payload.id, Some(vec![_patch]))
        .await
        .context(format!("{uuid}:"))?;
    Ok(())
}
