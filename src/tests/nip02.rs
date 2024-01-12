use crate::tests::prelude::*;

static CONTACT_LIST_TEST_INTERNAL_SUBSCRIPTION_ID: Lazy<InternalSubscriptionId> =
    Lazy::new(|| InternalSubscriptionId::Custom("contact_list_test".to_owned()));

const SUBSCRIPTION_NAME: &str = "contact list";

pub async fn test(client: &NostrClient, relay: &Relay) -> TestReport {
    let span = span!(Level::INFO, "nip02: set contact list").entered();

    let mut logger = Logger::new(Nips::Nip02);

    let event_subscription: Option<(SubscriptionId, Timestamp)> = establish_subscription(
        SUBSCRIPTION_NAME,
        relay,
        CONTACT_LIST_TEST_INTERNAL_SUBSCRIPTION_ID.clone(),
        nostr::Filter::new()
            .author(client.keys().await.public_key())
            .kind(Kind::ContactList),
        &mut logger,
    )
    .await;

    todo!("publish contact list + ensure recved by subscription + fetch and verify");

    drop(span);

    TestReport::Passed(Nips::Nip02)
}
