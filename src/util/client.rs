use aws_sdk_glacier::client::Client as GlacierClient;
use aws_config::{BehaviorVersion, Region};
use super::vault::GlacierVaultSpec;

pub async fn get_client(vault_spec: &GlacierVaultSpec) -> GlacierClient {
    let region = Region::new(vault_spec.region().to_owned());
    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(region)
        .load()
        .await;
    GlacierClient::new(&config)
}
