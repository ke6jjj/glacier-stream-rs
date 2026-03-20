use super::vault::GlacierVaultSpec;
use aws_config::{BehaviorVersion, Region};
use aws_sdk_glacier::client::Client as GlacierClient;

pub async fn get_client(vault_spec: &GlacierVaultSpec) -> GlacierClient {
    let region = Region::new(vault_spec.region().to_owned());
    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(region)
        .load()
        .await;
    GlacierClient::new(&config)
}
