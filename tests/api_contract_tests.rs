use serde::Deserialize;
use serde_yaml::{Mapping, Value};
use std::fs;

#[derive(Debug, Deserialize)]
struct ApiContractFixture {
    required_paths: Vec<String>,
    required_envelope_fields: Vec<String>,
    required_hop_fields: Vec<String>,
    required_probe_summary_fields: Vec<String>,
    required_out_of_scope_cli_flags: Vec<String>,
}

fn load_fixture() -> ApiContractFixture {
    let fixture_raw = fs::read_to_string("tests/fixtures/api_openapi_required_paths.yaml")
        .expect("failed to read api contract fixture");
    serde_yaml::from_str(&fixture_raw).expect("failed to parse api contract fixture")
}

fn load_openapi() -> Value {
    let raw =
        fs::read_to_string("docs/api/openapi.yaml").expect("failed to read docs/api/openapi.yaml");
    serde_yaml::from_str(&raw).expect("failed to parse docs/api/openapi.yaml as yaml")
}

fn as_mapping(value: &Value) -> &Mapping {
    value.as_mapping().expect("expected yaml mapping")
}

fn map_get<'a>(map: &'a Mapping, key: &str) -> &'a Value {
    map.get(Value::String(key.to_string()))
        .unwrap_or_else(|| panic!("missing key: {key}"))
}

fn map_get_optional<'a>(map: &'a Mapping, key: &str) -> Option<&'a Value> {
    map.get(Value::String(key.to_string()))
}

fn required_property_names(schema: &Value) -> Vec<String> {
    let schema_map = as_mapping(schema);
    let required = map_get(schema_map, "required")
        .as_sequence()
        .expect("schema.required should be a sequence");

    required
        .iter()
        .map(|item| {
            item.as_str()
                .expect("required entries should be strings")
                .to_string()
        })
        .collect()
}

fn property_names(schema: &Value) -> Vec<String> {
    let schema_map = as_mapping(schema);
    let properties = as_mapping(map_get(schema_map, "properties"));

    properties
        .keys()
        .map(|item| {
            item.as_str()
                .expect("property entries should be strings")
                .to_string()
        })
        .collect()
}

fn schema_ref_at_operation_response(
    paths: &Mapping,
    path: &str,
    method: &str,
    status_code: &str,
) -> String {
    let operation = as_mapping(map_get(as_mapping(map_get(paths, path)), method));
    let responses = as_mapping(map_get(operation, "responses"));
    let response = as_mapping(map_get(responses, status_code));
    let content = as_mapping(map_get(response, "content"));
    let app_json = as_mapping(map_get(content, "application/json"));
    map_get(app_json, "schema")
        .as_mapping()
        .and_then(|schema| schema.get(Value::String("$ref".to_string())))
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("missing schema $ref for {method} {path} {status_code}"))
        .to_string()
}

#[test]
fn openapi_contract_has_required_paths_and_fields() {
    let fixture = load_fixture();

    let openapi = load_openapi();
    let openapi_map = as_mapping(&openapi);

    let paths = as_mapping(map_get(openapi_map, "paths"));
    for required_path in &fixture.required_paths {
        assert!(
            map_get_optional(paths, required_path).is_some(),
            "missing required path: {required_path}"
        );
    }

    let components = as_mapping(map_get(openapi_map, "components"));
    let schemas = as_mapping(map_get(components, "schemas"));

    let envelope_meta_required = required_property_names(map_get(schemas, "EnvelopeMeta"));
    for required in &fixture.required_envelope_fields {
        assert!(
            envelope_meta_required.iter().any(|field| field == required),
            "EnvelopeMeta.required must contain {required}"
        );
    }

    let hop_required = required_property_names(map_get(schemas, "HopResult"));
    for required in &fixture.required_hop_fields {
        assert!(
            hop_required.iter().any(|field| field == required),
            "HopResult.required must contain {required}"
        );
    }

    let summary_required = required_property_names(map_get(schemas, "ProbeSummary"));
    for required in &fixture.required_probe_summary_fields {
        assert!(
            summary_required.iter().any(|field| field == required),
            "ProbeSummary.required must contain {required}"
        );
    }
}

#[test]
fn openapi_contract_enforces_protocol_port_rules() {
    let openapi = load_openapi();
    let openapi_map = as_mapping(&openapi);
    let components = as_mapping(map_get(openapi_map, "components"));
    let schemas = as_mapping(map_get(components, "schemas"));

    let create_request = as_mapping(map_get(schemas, "CreateProbeRequest"));
    let one_of = map_get(create_request, "oneOf")
        .as_sequence()
        .expect("CreateProbeRequest.oneOf must be present")
        .len();
    assert_eq!(
        one_of, 2,
        "CreateProbeRequest should have ICMP and TCP/UDP variants"
    );

    let icmp_required = required_property_names(map_get(schemas, "CreateProbeRequestIcmp"));
    assert!(
        icmp_required.iter().any(|field| field == "targets"),
        "CreateProbeRequestIcmp must require targets"
    );
    assert!(
        !icmp_required.iter().any(|field| field == "port"),
        "CreateProbeRequestIcmp must not require port"
    );

    let tcp_udp_required = required_property_names(map_get(schemas, "CreateProbeRequestTcpUdp"));
    for required in ["targets", "protocol", "port"] {
        assert!(
            tcp_udp_required.iter().any(|field| field == required),
            "CreateProbeRequestTcpUdp must require {required}"
        );
    }

    assert!(
        !tcp_udp_required
            .iter()
            .any(|field| field == "timeout_seconds"),
        "CreateProbeRequestTcpUdp must not require optional tuning fields"
    );
}

#[test]
fn openapi_contract_uses_envelope_across_success_and_error() {
    let openapi = load_openapi();
    let openapi_map = as_mapping(&openapi);
    let paths = as_mapping(map_get(openapi_map, "paths"));

    assert_eq!(
        schema_ref_at_operation_response(paths, "/api/v1/health", "get", "200"),
        "#/components/schemas/HealthResponse"
    );
    assert_eq!(
        schema_ref_at_operation_response(paths, "/api/v1/probes", "post", "202"),
        "#/components/schemas/CreateProbeResponse"
    );
    assert_eq!(
        schema_ref_at_operation_response(paths, "/api/v1/probes", "post", "400"),
        "#/components/schemas/ErrorResponse"
    );
    assert_eq!(
        schema_ref_at_operation_response(paths, "/api/v1/probes/{id}", "get", "200"),
        "#/components/schemas/GetProbeResponse"
    );
    assert_eq!(
        schema_ref_at_operation_response(paths, "/api/v1/probes/{id}", "get", "400"),
        "#/components/schemas/ErrorResponse"
    );
    assert_eq!(
        schema_ref_at_operation_response(paths, "/api/v1/probes/{id}", "get", "404"),
        "#/components/schemas/ErrorResponse"
    );
}

#[test]
fn openapi_contract_documents_error_shape_fields_and_required() {
    let openapi = load_openapi();
    let openapi_map = as_mapping(&openapi);
    let components = as_mapping(map_get(openapi_map, "components"));
    let schemas = as_mapping(map_get(components, "schemas"));

    let error_response_required = required_property_names(map_get(schemas, "ErrorResponse"));
    assert!(
        error_response_required.iter().any(|field| field == "meta"),
        "ErrorResponse must require meta"
    );
    assert!(
        error_response_required.iter().any(|field| field == "error"),
        "ErrorResponse must require error"
    );

    let error_required = required_property_names(map_get(schemas, "ApiProblemDetails"));
    for required in ["type", "title", "status", "detail", "code"] {
        assert!(
            error_required.iter().any(|field| field == required),
            "ApiProblemDetails.required must contain {required}"
        );
    }

    let error_properties = property_names(map_get(schemas, "ApiProblemDetails"));
    for property in ["type", "title", "status", "detail", "code"] {
        assert!(
            error_properties.iter().any(|field| field == property),
            "ApiProblemDetails.properties must contain {property}"
        );
    }
}

#[test]
fn openapi_contract_documents_cli_only_v1_out_of_scope() {
    let fixture = load_fixture();

    let raw =
        fs::read_to_string("docs/api/openapi.yaml").expect("failed to read docs/api/openapi.yaml");

    for flag in &fixture.required_out_of_scope_cli_flags {
        assert!(
            raw.contains(flag),
            "openapi description must document CLI-only out-of-scope flag: {flag}"
        );
    }
}

fn sorted(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values
}

fn create_probe_request_property_names(openapi: &Value, schema_name: &str) -> Vec<String> {
    let openapi_map = as_mapping(openapi);
    let components = as_mapping(map_get(openapi_map, "components"));
    let schemas = as_mapping(map_get(components, "schemas"));
    property_names(map_get(schemas, schema_name))
}

#[test]
fn openapi_create_probe_request_fields_match_rust_dto_fields() {
    use windows_mtr::service::api_models::{ApiProbeProtocol, CreateProbeRequestDto};

    let dto = CreateProbeRequestDto {
        targets: Vec::new(),
        protocol: ApiProbeProtocol::Icmp,
        port: None,
        count: None,
        max_hops: None,
        resolve_dns: None,
        include_asn: None,
        interval_seconds: None,
        timeout_seconds: None,
    };

    let dto_value = serde_json::to_value(dto).expect("dto should serialize");
    let dto_fields = sorted(
        dto_value
            .as_object()
            .expect("dto should serialize to object")
            .keys()
            .cloned()
            .collect(),
    );

    let openapi = load_openapi();

    let tcp_udp_fields = sorted(create_probe_request_property_names(
        &openapi,
        "CreateProbeRequestTcpUdp",
    ));
    assert_eq!(
        dto_fields, tcp_udp_fields,
        "CreateProbeRequestTcpUdp properties must match Rust DTO fields"
    );

    let mut icmp_expected_fields = dto_fields.clone();
    icmp_expected_fields.retain(|field| field != "port");
    let icmp_fields = sorted(create_probe_request_property_names(
        &openapi,
        "CreateProbeRequestIcmp",
    ));
    assert_eq!(
        icmp_expected_fields, icmp_fields,
        "CreateProbeRequestIcmp properties must match Rust DTO fields except protocol-specific port"
    );
}
