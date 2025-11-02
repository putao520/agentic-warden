use agentic_warden::sync::oauth_client::OAuthClient;

/// Integration test scaffold that can be enabled locally once real Google OAuth
/// credentials are available. It exercises the live token refresh endpoint.
#[tokio::test]
#[ignore = "Requires GOOGLE_CLIENT_ID, GOOGLE_CLIENT_SECRET, and GOOGLE_REFRESH_TOKEN environment variables"]
async fn refresh_token_flow_uses_real_google_api() {
    let (Ok(client_id), Ok(client_secret), Ok(refresh_token)) = (
        std::env::var("GOOGLE_CLIENT_ID"),
        std::env::var("GOOGLE_CLIENT_SECRET"),
        std::env::var("GOOGLE_REFRESH_TOKEN"),
    ) else {
        eprintln!("Skipping real OAuth integration test (missing GOOGLE_* credentials)");
        return;
    };

    let mut client = OAuthClient::new(client_id, client_secret, Some(refresh_token));
    let response = client
        .refresh_access_token()
        .await
        .expect("Refresh token flow should succeed with valid credentials");

    assert!(
        !response.access_token.is_empty(),
        "Received empty access token from Google"
    );
}
