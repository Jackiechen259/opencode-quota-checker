use opencode_core::{OpenCodeClient, OpenCodeError, QuotaService};
use std::net::TcpListener;
use std::time::Duration;
use url::Url;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const USAGE_OK: &str = include_str!("../../../tests/fixtures/opencode-go/usage-ok.json");
const USAGE_LIMITED: &str = include_str!("../../../tests/fixtures/opencode-go/usage-limited.json");
const USAGE_MALFORMED: &str =
    include_str!("../../../tests/fixtures/opencode-go/usage-malformed.json");

const USAGE_PATH: &str = "/zen/go/v1/usage";
const CONSOLE_USAGE_PATH: &str = "/console/api/usage/summary";
const GO_STATUS_PATH: &str = "/console/api/go/status";
const GO_ACTIVITY_PATH: &str = "/console/api/activity/go";
const BILLING_STATUS_PATH: &str = "/console/api/billing/status";
const API_KEY: &str = "oc_sk_test_123";
const WORKSPACE_ID: &str = "workspace-test-123";
const TOKEN_USAGE_OK: &str =
    include_str!("../../../tests/fixtures/opencode-console/usage-summary.json");
const GO_STATUS_OK: &str = include_str!("../../../tests/fixtures/opencode-go/go-status.json");
const GO_ACTIVITY_OK: &str = include_str!("../../../tests/fixtures/opencode-go/go-activity.json");
const BILLING_STATUS_OK: &str =
    include_str!("../../../tests/fixtures/opencode-go/billing-status.json");

async fn client(server: &MockServer) -> OpenCodeClient {
    OpenCodeClient::with_endpoint(
        server
            .uri()
            .parse::<Url>()
            .expect("mock server URL is valid"),
    )
    .expect("mock endpoint creates a client")
}

#[tokio::test]
async fn fetches_usage_with_a_bearer_token() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(USAGE_PATH))
        .and(header("accept", "application/json"))
        .and(header("authorization", format!("Bearer {API_KEY}")))
        .respond_with(ResponseTemplate::new(200).set_body_string(USAGE_OK))
        .expect(1)
        .mount(&server)
        .await;

    let body = client(&server)
        .await
        .fetch_usage(API_KEY, Some(WORKSPACE_ID))
        .await
        .expect("mock usage endpoint is successful");
    assert!(body.contains("resetsAt"));
}

#[tokio::test]
async fn identifies_itself_and_sends_the_workspace_header() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(USAGE_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_string(USAGE_OK))
        .mount(&server)
        .await;

    client(&server)
        .await
        .fetch_usage(API_KEY, Some(WORKSPACE_ID))
        .await
        .expect("mock usage endpoint is successful");

    let requests = server
        .received_requests()
        .await
        .expect("wiremock records requests");
    let request = requests.first().expect("one request was sent");

    let user_agent = request
        .headers
        .get("user-agent")
        .expect("a user agent is always sent")
        .to_str()
        .expect("the user agent is ASCII");
    assert!(
        user_agent.starts_with("opencode-quota-checker/"),
        "unexpected user agent: {user_agent}"
    );
    assert_eq!(
        request
            .headers
            .get("x-opencode-org-id")
            .expect("the workspace header is sent when configured")
            .to_str()
            .expect("the workspace header is ASCII"),
        WORKSPACE_ID
    );
    assert!(
        !request.headers.contains_key("cookie"),
        "the retired auth cookie must never be sent"
    );
}

#[tokio::test]
async fn omits_the_workspace_header_when_it_is_not_configured() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(USAGE_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_string(USAGE_OK))
        .mount(&server)
        .await;

    client(&server)
        .await
        .fetch_usage(API_KEY, Some("   "))
        .await
        .expect("a blank workspace is simply omitted");

    let requests = server
        .received_requests()
        .await
        .expect("wiremock records requests");
    let request = requests.first().expect("one request was sent");
    assert!(
        !request.headers.contains_key("x-opencode-org-id"),
        "a blank workspace id must not produce a header"
    );
}

#[tokio::test]
async fn maps_statuses_to_typed_errors_without_leaking_the_key() {
    for (status, expected) in [
        (401, "authentication failed"),
        (403, "not subscribed"),
        (429, "rate limited"),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(status).set_body_string("nope"))
            .mount(&server)
            .await;

        let error = client(&server)
            .await
            .fetch_usage(API_KEY, Some(WORKSPACE_ID))
            .await
            .expect_err("non-success status must fail");

        let rendered = error.to_string();
        assert!(
            rendered.contains(expected),
            "expected {expected} in {rendered:?}"
        );
        assert!(
            !rendered.contains(API_KEY),
            "error must never contain the API key"
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
            .fetch_usage(API_KEY, None)
            .await
            .expect_err("5xx must fail");
        match error {
            OpenCodeError::Http {
                status: actual,
                body,
            } => {
                assert_eq!(actual.as_u16(), status);
                assert!(body.chars().count() < 550);
                assert!(!body.contains(API_KEY));
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
                .set_body_string(USAGE_OK),
        )
        .mount(&server)
        .await;
    let endpoint = server.uri().parse::<Url>().expect("mock URL parses");
    let client = OpenCodeClient::with_endpoint_and_timeout(endpoint, Duration::from_millis(25))
        .expect("test timeout creates a client");

    let error = client
        .fetch_usage(API_KEY, None)
        .await
        .expect_err("delayed response must time out");
    assert!(matches!(error, OpenCodeError::Request(ref source) if source.is_timeout()));
}

#[tokio::test]
async fn connection_refusal_is_a_request_error() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("test listener binds");
    let address = listener.local_addr().expect("test address");
    drop(listener);

    let endpoint = Url::parse(&format!("http://{address}/")).expect("URL parses");
    let client = OpenCodeClient::with_endpoint(endpoint).expect("endpoint has a host");

    let error = client
        .fetch_usage(API_KEY, None)
        .await
        .expect_err("refused connection must fail");
    assert!(matches!(error, OpenCodeError::Request(_)));
}

#[tokio::test]
async fn quota_service_normalizes_the_mock_usage_response() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(header("authorization", format!("Bearer {API_KEY}")))
        .respond_with(ResponseTemplate::new(200).set_body_string(USAGE_OK))
        .mount(&server)
        .await;

    let service = QuotaService::with_client(client(&server).await);
    let report = service
        .fetch_quota(API_KEY, Some(WORKSPACE_ID))
        .await
        .expect("mock pipeline is successful");

    assert_eq!(report.windows.len(), 3);
    assert_eq!(report.windows[0].key, "rolling-5h");
    assert_eq!(report.windows[0].percent, 78.0);
    assert_eq!(report.windows[0].remaining, 22.0);
    assert_eq!(report.windows[1].key, "weekly");
    assert_eq!(report.windows[2].key, "monthly");
    assert!(report.windows.iter().all(|window| !window.limited));
}

#[tokio::test]
async fn quota_service_marks_rate_limited_windows() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string(USAGE_LIMITED))
        .mount(&server)
        .await;

    let service = QuotaService::with_client(client(&server).await);
    let report = service
        .fetch_quota(API_KEY, None)
        .await
        .expect("the limited payload is valid");

    assert!(!report.windows[0].limited);
    assert!(report.windows[1].limited);
    assert_eq!(report.windows[1].percent, 100.0);
    assert_eq!(report.windows[1].remaining, 0.0);
    assert!(!report.windows[2].limited);
}

#[tokio::test]
async fn malformed_payload_is_a_parse_error_not_zero_usage() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string(USAGE_MALFORMED))
        .mount(&server)
        .await;

    let service = QuotaService::with_client(client(&server).await);
    let error = service
        .fetch_quota(API_KEY, None)
        .await
        .expect_err("an unknown payload must fail");
    assert!(matches!(error, OpenCodeError::Parse(_)));
}

#[tokio::test]
async fn raw_usage_returns_the_response_body() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string(USAGE_OK))
        .mount(&server)
        .await;

    let service = QuotaService::with_client(client(&server).await);
    let raw = service
        .fetch_raw_usage(API_KEY, Some(WORKSPACE_ID))
        .await
        .expect("the raw body is returned");
    assert_eq!(raw.trim(), USAGE_OK.trim());
}

#[tokio::test]
async fn quota_service_rejects_a_blank_api_key_without_a_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string(USAGE_OK))
        .mount(&server)
        .await;

    let service = QuotaService::with_client(client(&server).await);
    let error = service
        .fetch_quota("  ", Some(WORKSPACE_ID))
        .await
        .expect_err("a blank key must fail");
    assert!(matches!(error, OpenCodeError::CredentialsMissing));

    let requests = server
        .received_requests()
        .await
        .expect("wiremock records requests");
    assert!(
        requests.is_empty(),
        "a blank key must never reach the network"
    );
}

#[tokio::test]
async fn fetches_token_usage_with_the_console_summary_contract() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(CONSOLE_USAGE_PATH))
        .and(query_param("range", "30d"))
        .and(query_param("includeLegacyKeys", "true"))
        .and(header("accept", "application/json"))
        .and(header("authorization", format!("Bearer {API_KEY}")))
        .respond_with(ResponseTemplate::new(200).set_body_string(TOKEN_USAGE_OK))
        .expect(1)
        .mount(&server)
        .await;

    let body = client(&server)
        .await
        .fetch_token_usage(API_KEY)
        .await
        .expect("mock console endpoint is successful");
    assert!(body.contains("totalCostMicroCents"));

    let requests = server
        .received_requests()
        .await
        .expect("wiremock records requests");
    let request = requests.first().expect("one request was sent");
    let user_agent = request
        .headers
        .get("user-agent")
        .expect("a user agent is always sent")
        .to_str()
        .expect("the user agent is ASCII");
    assert!(
        user_agent.starts_with("opencode-quota-checker/"),
        "unexpected user agent: {user_agent}"
    );
    assert!(
        !request.headers.contains_key("x-opencode-org-id"),
        "the Console summary uses the key-bound workspace, not the Go header"
    );
    assert!(
        !request.headers.contains_key("x-org-id"),
        "a personal API key is already bound to its workspace"
    );
    assert!(
        !request.headers.contains_key("cookie"),
        "the retired auth cookie must never be sent"
    );
}

#[tokio::test]
async fn quota_service_parses_the_token_usage_summary() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(CONSOLE_USAGE_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_string(TOKEN_USAGE_OK))
        .mount(&server)
        .await;

    let service = QuotaService::with_client(client(&server).await);
    let report = service
        .fetch_token_usage(API_KEY)
        .await
        .expect("the summary is parsed");

    assert_eq!(report.range, "30d");
    assert_eq!(report.requests, 123);
    assert_eq!(report.input_tokens, 1000);
    assert_eq!(report.output_tokens, 2000);
    assert_eq!(report.cache_read_tokens, 3000);
    assert_eq!(report.cache_write_tokens, 100);
    assert_eq!(report.total_tokens, 6100);
    assert_eq!(report.cost_microcents, 30_000_000);
}

#[tokio::test]
async fn maps_token_usage_statuses_to_typed_errors_without_leaking_the_key() {
    for (status, expected) in [
        (401, "authentication failed"),
        (403, "cannot read workspace usage"),
        (429, "rate limited"),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(CONSOLE_USAGE_PATH))
            .respond_with(ResponseTemplate::new(status).set_body_string("nope"))
            .mount(&server)
            .await;

        let error = client(&server)
            .await
            .fetch_token_usage(API_KEY)
            .await
            .expect_err("non-success status must fail");

        let rendered = error.to_string();
        assert!(
            rendered.contains(expected),
            "expected {expected} in {rendered:?}"
        );
        assert!(
            !rendered.contains(API_KEY),
            "error must never contain the API key"
        );
        if status == 403 {
            assert!(matches!(error, OpenCodeError::UsagePermissionDenied));
        }
    }
}

#[tokio::test]
async fn malformed_token_usage_is_a_parse_error_not_zero_usage() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(CONSOLE_USAGE_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"totalRequests":"1"}"#))
        .mount(&server)
        .await;

    let service = QuotaService::with_client(client(&server).await);
    let error = service
        .fetch_token_usage(API_KEY)
        .await
        .expect_err("a partial summary must fail");
    assert!(matches!(error, OpenCodeError::Parse(_)));
}

#[tokio::test]
async fn token_usage_rejects_a_blank_api_key_without_a_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string(TOKEN_USAGE_OK))
        .mount(&server)
        .await;

    let service = QuotaService::with_client(client(&server).await);
    let error = service
        .fetch_token_usage("  ")
        .await
        .expect_err("a blank key must fail");
    assert!(matches!(error, OpenCodeError::CredentialsMissing));

    let requests = server
        .received_requests()
        .await
        .expect("wiremock records requests");
    assert!(
        requests.is_empty(),
        "a blank key must never reach the network"
    );
}

#[tokio::test]
async fn fetches_go_status_with_fresh_and_the_expected_headers() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(GO_STATUS_PATH))
        .and(query_param("fresh", "true"))
        .and(header("accept", "application/json"))
        .and(header("authorization", format!("Bearer {API_KEY}")))
        .respond_with(ResponseTemplate::new(200).set_body_string(GO_STATUS_OK))
        .expect(1)
        .mount(&server)
        .await;

    let body = client(&server)
        .await
        .fetch_go_status(API_KEY, true)
        .await
        .expect("mock Go status endpoint is successful");
    assert!(body.contains("useBalance"));

    let requests = server
        .received_requests()
        .await
        .expect("wiremock records requests");
    let request = requests.first().expect("one request was sent");
    assert!(
        !request.headers.contains_key("x-opencode-org-id"),
        "the Console status endpoint uses the key-bound workspace"
    );
    let user_agent = request
        .headers
        .get("user-agent")
        .expect("a user agent is always sent")
        .to_str()
        .expect("the user agent is ASCII");
    assert!(user_agent.starts_with("opencode-quota-checker/"));
}

#[tokio::test]
async fn quota_service_parses_the_go_status_meters() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(GO_STATUS_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_string(GO_STATUS_OK))
        .mount(&server)
        .await;

    let service = QuotaService::with_client(client(&server).await);
    let status = service
        .fetch_go_status(API_KEY, false)
        .await
        .expect("status is parsed")
        .expect("subscription is active");

    assert_eq!(status.product, "go");
    assert!(!status.use_balance);
    let month = status.access.expect("access").month.expect("monthly meter");
    assert_eq!(month.percent, 24.0);
    assert_eq!(month.used_microcents, 14_400_000_000);
    assert!(!month.limited);
}

#[tokio::test]
async fn quota_service_parses_go_activity_totals() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(GO_ACTIVITY_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_string(GO_ACTIVITY_OK))
        .mount(&server)
        .await;

    let service = QuotaService::with_client(client(&server).await);
    let activity = service
        .fetch_go_activity(API_KEY)
        .await
        .expect("activity is parsed")
        .expect("activity exists");

    assert_eq!(activity.daily.len(), 2);
    assert_eq!(activity.total_requests, 5);
    assert_eq!(activity.total_tokens, 4000);
    assert_eq!(activity.total_cost_microcents, 300);
    assert_eq!(activity.models.len(), 2);
}

#[tokio::test]
async fn quota_service_parses_the_billing_credit() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(BILLING_STATUS_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_string(BILLING_STATUS_OK))
        .mount(&server)
        .await;

    let service = QuotaService::with_client(client(&server).await);
    let billing = service
        .fetch_billing_status(API_KEY)
        .await
        .expect("billing is parsed");
    assert_eq!(billing.available_microcents, 0);
    assert_eq!(billing.billing_mode, "prepaid");
}

#[tokio::test]
async fn go_status_and_activity_forbidden_are_usage_permission_errors() {
    for endpoint in [GO_STATUS_PATH, GO_ACTIVITY_PATH] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(endpoint))
            .respond_with(ResponseTemplate::new(403))
            .mount(&server)
            .await;

        let client = client(&server).await;
        let error = if endpoint == GO_STATUS_PATH {
            client.fetch_go_status(API_KEY, false).await
        } else {
            client.fetch_go_activity(API_KEY).await
        }
        .expect_err("403 must fail");
        assert!(matches!(error, OpenCodeError::UsagePermissionDenied));
    }
}

#[tokio::test]
async fn billing_forbidden_has_its_own_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(BILLING_STATUS_PATH))
        .respond_with(ResponseTemplate::new(403))
        .mount(&server)
        .await;

    let error = client(&server)
        .await
        .fetch_billing_status(API_KEY)
        .await
        .expect_err("403 must fail");
    assert!(matches!(error, OpenCodeError::BillingPermissionDenied));
    assert!(!error.to_string().contains(API_KEY));
}

#[tokio::test]
async fn null_go_status_and_activity_are_valid_empty_values() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(GO_STATUS_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_string("null"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(GO_ACTIVITY_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_string("null"))
        .mount(&server)
        .await;

    let service = QuotaService::with_client(client(&server).await);
    assert!(service
        .fetch_go_status(API_KEY, false)
        .await
        .expect("null is valid")
        .is_none());
    assert!(service
        .fetch_go_activity(API_KEY)
        .await
        .expect("null is valid")
        .is_none());
}
