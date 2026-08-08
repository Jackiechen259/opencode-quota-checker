use std::net::TcpListener;
use std::time::Duration;
use url::Url;
use opencode_core::opencode::{OpenCodeGoClient, OpenCodeGoProvider};
use opencode_core::{Provider, VolcError};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const SSR: &str = include_str!("../../../tests/fixtures/opencode-go/ssr-dashboard.html");
const LOGIN: &str = include_str!("../../../tests/fixtures/opencode-go/login-page.html");

const WORKSPACE_ID: &str = "workspace-test-123";
const AUTH_COOKIE: &str = "test-auth-cookie";

async fn client(server: &MockServer) -> OpenCodeGoClient {
    OpenCodeGoClient::with_endpoint(
        server
            .uri()
            .parse::<Url>()
            .expect("mock server URL is valid"),
    )
    .expect("mock endpoint creates a client")
}

#[tokio::test]
async fn fetches_dashboard_with_cookie_and_returns_html() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/workspace/workspace-test-123/go"))
        .and(header("accept", "text/html"))
        .and(header("cookie", "auth=test-auth-cookie"))
        .respond_with(ResponseTemplate::new(200).set_body_string(SSR))
        .expect(1)
        .mount(&server)
        .await;

    let body = client(&server)
        .await
        .fetch_dashboard(WORKSPACE_ID, AUTH_COOKIE)
        .await
        .expect("mock dashboard is successful");
    assert!(body.contains("usagePercent"));
}

#[tokio::test]
async fn returns_login_html_and_parser_classifies_it() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string(LOGIN))
        .mount(&server)
        .await;

    let body = client(&server)
        .await
        .fetch_dashboard(WORKSPACE_ID, AUTH_COOKIE)
        .await
        .expect("200 login HTML is still a successful fetch");
    let error = opencode_core::opencode::parser::parse_open_code_go_quota(
        &body,
        chrono::Utc::now().timestamp_millis(),
    )
    .expect_err("login page must be classified as authentication failure");
    assert!(matches!(error, VolcError::AuthenticationFailed));
}

#[tokio::test]
async fn maps_authentication_and_not_found_statuses() {
    for (status, expected) in [
        (401, "authentication failed"),
        (403, "authentication failed"),
        (404, "workspace not found"),
        (429, "request rate limited"),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(status).set_body_string("nope"))
            .mount(&server)
            .await;

        let error = client(&server)
            .await
            .fetch_dashboard(WORKSPACE_ID, AUTH_COOKIE)
            .await
            .expect_err("non-success status must fail");
        let rendered = error.to_string();
        assert!(
            rendered.contains(expected),
            "expected {expected} in {rendered:?}"
        );
        assert!(
            !rendered.contains(AUTH_COOKIE),
            "error must never contain the auth cookie"
        );
    }
}

#[tokio::test]
async fn maps_server_errors_to_http_with_bounded_body() {
    for status in [500, 503] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(status).set_body_string("x".repeat(800)))
            .mount(&server)
            .await;

        let error = client(&server)
            .await
            .fetch_dashboard(WORKSPACE_ID, AUTH_COOKIE)
            .await
            .expect_err("5xx must fail");
        match error {
            VolcError::Http {
                status: actual,
                body,
            } => {
                assert_eq!(actual.as_u16(), status);
                assert!(body.chars().count() < 550);
                assert!(!body.contains(AUTH_COOKIE));
            }
            other => panic!("expected HTTP error, got {other}"),
        }
    }
}

#[tokio::test]
async fn enforces_request_timeout() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_millis(150))
                .set_body_string(SSR),
        )
        .mount(&server)
        .await;
    let endpoint = server.uri().parse::<Url>().expect("mock URL parses");
    let client = OpenCodeGoClient::with_endpoint_and_timeout(endpoint, Duration::from_millis(25))
        .expect("test timeout creates a client");

    let error = client
        .fetch_dashboard(WORKSPACE_ID, AUTH_COOKIE)
        .await
        .expect_err("delayed response must time out");
    assert!(matches!(error, VolcError::Request(ref source) if source.is_timeout()));
}

#[tokio::test]
async fn connection_refusal_is_a_request_error() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("test listener binds");
    let address = listener.local_addr().expect("test address");
    drop(listener);

    let endpoint = Url::parse(&format!("http://{address}/")).expect("URL parses");
    let client = OpenCodeGoClient::with_endpoint(endpoint).expect("endpoint has a host");

    let error = client
        .fetch_dashboard(WORKSPACE_ID, AUTH_COOKIE)
        .await
        .expect_err("refused connection must fail");
    assert!(matches!(error, VolcError::Request(_)));
}

#[tokio::test]
async fn provider_pipeline_normalizes_the_mock_dashboard() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(header("cookie", "auth=test-auth-cookie"))
        .respond_with(ResponseTemplate::new(200).set_body_string(SSR))
        .mount(&server)
        .await;

    let provider = OpenCodeGoProvider::with_client(client(&server).await);
    let report = provider
        .fetch_quota(WORKSPACE_ID, AUTH_COOKIE)
        .await
        .expect("mock pipeline is successful");

    assert_eq!(report.provider, Provider::OpenCodeGo);
    assert_eq!(report.windows.len(), 3);
    assert_eq!(report.windows[0].key, "rolling-5h");
    assert_eq!(report.windows[0].percent, 78.0);
    assert_eq!(report.windows[0].remaining, 22.0);
    assert_eq!(report.windows[1].key, "weekly");
    assert_eq!(report.windows[2].key, "monthly");
}

#[tokio::test]
async fn provider_rejects_empty_configuration() {
    let provider = OpenCodeGoProvider::default();
    let missing_workspace = provider
        .fetch_quota("  ", AUTH_COOKIE)
        .await
        .expect_err("empty workspace must fail");
    assert!(matches!(missing_workspace, VolcError::CredentialsMissing));

    let missing_cookie = provider
        .fetch_quota(WORKSPACE_ID, "  ")
        .await
        .expect_err("empty cookie must fail");
    assert!(matches!(missing_cookie, VolcError::CredentialsMissing));
}
