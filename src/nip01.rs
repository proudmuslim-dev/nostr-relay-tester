use nostr::{Kind, Timestamp};
use nostr_sdk::Client as NostrClient;
use tracing::{error, info, span, Level};

pub async fn test(client: &NostrClient) {
    let span = span!(Level::INFO, "nip01: publishing event");
    let _s = span.enter();

    // TODO: Get access to relay URL so as to utilize `subscribe_with_internal_id`
    client
        .subscribe(vec![nostr::Filter::new()
            .pubkey(client.keys().await.public_key())
            .kind(Kind::TextNote)
            .since(Timestamp::now())])
        .await;

    match client.publish_text_note("", vec![]).await {
        Ok(id) => info!("successfully published: {id}"),
        Err(e) => error!("failed to publish event: {e}"),
    }

    drop(_s);
    drop(span);
}
