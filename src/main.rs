extern crate core;

// The simplest loadtest example
mod graphql;
mod oidc;
mod restapi;
mod website;

use crate::{
    graphql::graphql_query_advisory,
    oidc::{OpenIdTokenProvider, OpenIdTokenProviderConfigArguments},
    restapi::*,
    website::*,
};
use anyhow::Context;
use goose::prelude::*;
use std::{str::FromStr, sync::Arc, time::Duration};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let wait_time_from: u64 = std::env::var("WAIT_TIME_FROM")
        .map(|s| s.parse().unwrap_or(5))
        .unwrap_or(5);
    let wait_time_to: u64 = std::env::var("WAIT_TIME_TO")
        .map(|s| s.parse().unwrap_or(15))
        .unwrap_or(15);

    let provider = create_oidc_provider().await?;
    let custom_client = Transaction::new(Arc::new(move |user| {
        let provider = provider.clone();
        Box::pin(async move { setup_custom_client(&provider, user).await })
    }));

    GooseAttack::initialize()?
        .register_scenario(
            scenario!("WebsiteUser")
                // .set_weight(1)?
                .register_transaction(custom_client.clone().set_name("logon"))
                // After each transactions runs, sleep randomly from 5 to 15 seconds.
                .set_wait_time(
                    Duration::from_secs(wait_time_from),
                    Duration::from_secs(wait_time_to),
                )?
                .register_transaction(transaction!(website_index))
                .register_transaction(transaction!(website_openapi))
                .register_transaction(transaction!(website_sboms))
                .register_transaction(transaction!(website_packages))
                .register_transaction(transaction!(website_advisories))
                .register_transaction(transaction!(website_importers)),
        )
        .register_scenario(
            scenario!("RestAPIUser")
                // .set_weight(1)?
                .register_transaction(custom_client.clone().set_name("logon"))
                // After each transactions runs, sleep randomly from 5 to 15 seconds.
                .set_wait_time(
                    Duration::from_secs(wait_time_from),
                    Duration::from_secs(wait_time_to),
                )?
                .register_transaction(transaction!(list_organizations))
                .register_transaction(transaction!(list_advisory))
                .register_transaction(transaction!(list_advisory_paginated))
                .register_transaction(transaction!(get_advisory_by_doc_id))
                .register_transaction(transaction!(list_vulnerabilities))
                .register_transaction(transaction!(list_vulnerabilities_paginated))
                .register_transaction(transaction!(list_importer))
                .register_transaction(transaction!(list_packages))
                .register_transaction(transaction!(list_packages_paginated))
                .register_transaction(transaction!(search_packages))
                .register_transaction(transaction!(list_products))
                .register_transaction(transaction!(list_sboms))
                .register_transaction(transaction!(list_sboms_paginated)),
        )
        .register_scenario(
            scenario!("GraphQLUser")
                // .set_weight(1)?
                .register_transaction(custom_client.set_name("logon"))
                // After each transactions runs, sleep randomly from 5 to 15 seconds.
                .set_wait_time(
                    Duration::from_secs(wait_time_from),
                    Duration::from_secs(wait_time_to),
                )?
                .register_transaction(
                    transaction!(graphql_query_advisory).set_name("query advisory with /graphql"),
                ),
        )
        .execute()
        .await?;

    Ok(())
}

async fn create_oidc_provider() -> anyhow::Result<OpenIdTokenProvider> {
    let issuer_url = std::env::var("ISSUER_URL").context("Missing env-var 'ISSUER_URL'")?;
    let client_id = std::env::var("CLIENT_ID").context("Missing env-var 'CLIENT_ID'")?;
    let client_secret =
        std::env::var("CLIENT_SECRET").context("Missing env-var 'CLIENT_SECRET'")?;
    let refresh_before = std::env::var("OIDC_REFRESH_BEFORE").unwrap_or_else(|_| "30s".to_string());
    let refresh_before =
        humantime::Duration::from_str(&refresh_before).context("OIDC_REFRESH_BEFORE must parse")?;

    let provider = OpenIdTokenProvider::with_config(OpenIdTokenProviderConfigArguments {
        client_id,
        client_secret,
        issuer_url,
        refresh_before,
        tls_insecure: false,
    })
    .await
    .context("discover OIDC client")?;

    Ok(provider)
}

// required until https://github.com/tag1consulting/goose/pull/605 is merged
#[allow(clippy::expect_used)]
async fn setup_custom_client(
    provider: &OpenIdTokenProvider,
    user: &mut GooseUser,
) -> TransactionResult {
    set_custom_client(provider, user)
        .await
        .expect("Failed to set up client");
    Ok(())
}

async fn set_custom_client(
    provider: &OpenIdTokenProvider,
    user: &mut GooseUser,
) -> anyhow::Result<()> {
    use reqwest::{header, Client};

    log::debug!("Creating a new custom client");

    let auth_token: String = provider
        .provide_token()
        .await
        .context("get OIDC token")?
        .access_token;

    let mut headers = header::HeaderMap::new();
    headers.insert(
        "Authorization",
        header::HeaderValue::from_str(&format!("Bearer {auth_token}"))?,
    );

    // Build a custom client.
    let builder = Client::builder()
        .default_headers(headers)
        .user_agent("loadtest-ua")
        .timeout(Duration::from_secs(30));

    // Assign the custom client to this GooseUser.
    user.set_client_builder(builder).await?;

    Ok(())
}
