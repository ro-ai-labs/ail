use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use eigl::{
    build_program, check_document,
    collections::collection_path_value,
    core_model::json_string,
    eig_ir::{lower_document, run_bytecode, run_bytecode_with_operation_outputs},
    interpreter::simulate_with_operation_outputs,
    parse_rif_file, parse_rif_text, parse_rsl_text, predicate, render_rif_document, simulate,
    views::{effect_view, failure_view, flow_view, permission_view, view_model},
};

fn fixture(name: &str) -> String {
    format!("{}/examples/{name}", env!("CARGO_MANIFEST_DIR"))
}

fn temp_path(name: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("eigl-{name}-{suffix}.state"))
}

fn free_listen_addr() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().to_string()
}

fn server_test_guard() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

#[test]
fn parser_reads_confirm_order_rif() {
    let document = parse_rif_file(fixture("confirm_order.rif.md")).unwrap();

    assert_eq!(document.intent.name, "ConfirmOrder");
    assert_eq!(document.intent.subjects["order"].type_name, "Order");
    assert_eq!(document.intent.requires[0].text, "order.status is Draft");
    assert_eq!(
        document.intent.state_transitions[0].field_path,
        "order.status"
    );
    assert_eq!(document.intent.state_transitions[0].from_state, "Draft");
    assert_eq!(document.intent.state_transitions[0].to_state, "Confirmed");
    assert_eq!(document.intent.step_schedule, "sequential");
    assert_eq!(document.intent.steps.len(), 4);

    let capture_payment = &document.intent.steps[1];
    assert_eq!(capture_payment.number, 2);
    assert_eq!(capture_payment.title, "Capture payment");
    assert_eq!(
        capture_payment.call.as_ref().unwrap().target,
        "PaymentGateway.capture"
    );
    assert_eq!(
        capture_payment.call.as_ref().unwrap().args,
        vec!["order.payment_method", "order.total"]
    );
    assert_eq!(
        capture_payment.outputs["payment"].type_name,
        "PaymentCapture"
    );
    assert_eq!(capture_payment.changes, vec!["PaymentLedger"]);
    assert_eq!(capture_payment.external_calls, vec!["PaymentGateway"]);
    assert_eq!(capture_payment.may_fail, vec!["PaymentFailed"]);
    assert_eq!(document.intent.failure_handlers.len(), 2);
    assert_eq!(
        document.intent.failure_handlers[0].condition,
        "payment capture fails"
    );
    assert_eq!(
        document.intent.failure_handlers[0]
            .stop_failure
            .as_deref()
            .unwrap(),
        "PaymentFailed"
    );
    assert!(
        document.intent.guarantees[0]
            .statements
            .contains(&"order.status is Confirmed".to_string())
    );
}

#[test]
fn parser_elaborates_compact_rsl_into_rif() {
    let document = parse_rsl_text(
        r#"
        app Commerce

        things:
          thing Order
            field status: State<Draft, Confirmed, PaidAwaitingFulfillmentRetry>
            field items: List<OrderItem>
            field total: Int
            field payment_method: PaymentMethod

          thing OrderItem
            field id: Id<OrderItem>

          thing PaymentMethod
            field id: Id<PaymentMethod>

          thing PaymentLedger
            field id: Id<PaymentLedger>

          thing ShipmentQueue
            field id: Id<ShipmentQueue>

          thing InventoryReservation
            field id: Id<InventoryReservation>

          thing PaymentCapture
            field id: Id<PaymentCapture>

          thing ShipmentRequest
            field id: Id<ShipmentRequest>

        phrases:
          phrase "reserve inventory":
            means Inventory.reserve(order.items)
            produces reservation: InventoryReservation
            changes Inventory
            compensation is Inventory.release(reservation)

          phrase "capture payment":
            means PaymentGateway.capture(order.payment_method, order.total)
            produces payment: PaymentCapture
            changes PaymentLedger
            calls PaymentGateway
            may fail with PaymentFailed

          phrase "create shipment":
            means Shipping.create_request(order, reservation)
            produces shipment: ShipmentRequest
            changes ShipmentQueue
            calls Shipping
            may fail with ShipmentFailed

        intent Confirm order:
          subject:
            order: Order

          requires:
            order.status is Draft
            order.items.count > 0

          Draft -> Confirmed
          by reserve inventory, capture payment, create shipment
          on payment failure -> PaymentFailed
          on shipping failure -> PaidAwaitingFulfillmentRetry
        "#,
    )
    .unwrap();

    assert_eq!(document.intent.name, "ConfirmOrder");
    assert_eq!(document.intent.steps.len(), 4);
    assert_eq!(
        document.intent.steps[0].call.as_ref().unwrap().target,
        "Inventory.reserve"
    );
    assert_eq!(
        document.intent.steps[1].call.as_ref().unwrap().target,
        "PaymentGateway.capture"
    );
    assert_eq!(
        document.intent.failure_handlers[0].condition,
        "payment failure"
    );
    assert_eq!(
        document.intent.failure_handlers[1].stop_failure.as_deref(),
        Some("ShipmentFailed")
    );
    assert_eq!(
        document.intent.state_transitions[0].field_path,
        "order.status"
    );
    assert_eq!(check_document(&document), Vec::new());
}

#[test]
fn parser_elaborates_thing_and_state_sentences() {
    let document = parse_rsl_text(
        r#"
        intent Describe Domain

        things:
          A Customer has an email address.
          An Order can be Draft, Confirmed, Cancelled.

        subject:
          order: Order
        "#,
    )
    .unwrap();

    let customer = document.application.things.get("Customer").unwrap();
    assert_eq!(customer.fields["email_address"].type_name, "Text");

    let order = document.application.things.get("Order").unwrap();
    assert_eq!(
        order.fields["status"].type_name,
        "State<Draft, Confirmed, Cancelled>"
    );
}

#[test]
fn rif_imports_merge_app_fragments_by_path() {
    let imported_path = std::env::temp_dir().join("eigl-ticket-shared.rif.md");
    let root_path = std::env::temp_dir().join("eigl-ticket-root.rif.md");
    fs::write(
        &imported_path,
        r#"
        app TicketApi

        things:
          thing Ticket
            field id: Text
            field status: State<Open, Closed>

        intent ArchiveTicket

        subject:
          ticket: Ticket

        steps:
          1. Archive ticket
             set: ticket.status = Closed
        "#,
    )
    .unwrap();
    fs::write(
        &root_path,
        r#"
        imports:
          import eigl-ticket-shared.rif.md

        app TicketApi

        things:
          thing Report
            field ticket_id: Text

        intent CloseTicket

        subject:
          report: Report
          ticket: Ticket

        steps:
          1. Close ticket
             set: report.ticket_id = ticket.id
        "#,
    )
    .unwrap();

    let document = parse_rif_file(&root_path).unwrap();
    assert_eq!(
        document.application.things["Ticket"].fields["id"].type_name,
        "Text"
    );
    assert_eq!(
        document.application.things["Report"].fields["ticket_id"].type_name,
        "Text"
    );
    assert!(
        document
            .intents
            .iter()
            .any(|intent| intent.name == "ArchiveTicket")
    );
    assert!(
        document
            .intents
            .iter()
            .any(|intent| intent.name == "CloseTicket")
    );
    assert_eq!(check_document(&document), Vec::new());

    let rendered = render_rif_document(&document);
    let reparsed = parse_rif_text(&rendered).unwrap();
    let mut normalized_document = document.clone();
    normalized_document.source_path = None;
    assert_eq!(reparsed, normalized_document);

    let _ = fs::remove_file(imported_path);
    let _ = fs::remove_file(root_path);
}

#[test]
fn rif_imports_allow_distinct_app_names() {
    let imported_path = std::env::temp_dir().join("eigl-profile-shared.rif.md");
    let root_path = std::env::temp_dir().join("eigl-profile-root.rif.md");
    fs::write(
        &imported_path,
        r#"
        app ProfileLibrary

        things:
          thing Profile
            field email: Text

        intent SeedProfile

        subject:
          profile: Profile

        steps:
          1. Seed profile
             set: profile.email = alice@example.com
        "#,
    )
    .unwrap();
    fs::write(
        &root_path,
        r#"
        imports:
          import eigl-profile-shared.rif.md

        app Profiles

        things:
          thing Report
            field count: Int

        intent CountProfiles

        subject:
          report: Report

        steps:
          1. Count profiles
             set: report.count = 1
        "#,
    )
    .unwrap();

    let document = parse_rif_file(&root_path).unwrap();
    assert_eq!(document.application.name.as_deref(), Some("Profiles"));
    assert!(
        document
            .intents
            .iter()
            .any(|intent| intent.name == "SeedProfile")
    );

    let _ = fs::remove_file(imported_path);
    let _ = fs::remove_file(root_path);
}

#[test]
fn rif_import_aliases_intents_without_conflict() {
    let imported_path = std::env::temp_dir().join("eigl-order-actions-lib.rif.md");
    let root_path = std::env::temp_dir().join("eigl-order-actions-root.rif.md");
    fs::write(
        &imported_path,
        r#"
        app OrderLibrary

        things:
          thing Order
            field status: State<Open, Archived>

        collections:
          collection orders: Order

        intent ArchiveOrder

        subject:
          order: Order

        steps:
          1. Archive order
             set: order.status = Archived
        "#,
    )
    .unwrap();
    fs::write(
        &root_path,
        r#"
        imports:
          import eigl-order-actions-lib.rif.md as Shared
          import eigl-order-actions-lib.rif.md as Ops

        app Orders
        module Orders.Main

        things:
          thing Order
            field status: State<Open, Archived>

        intent ProcessOrder

        subject:
          order: Order

        steps:
          1. Open order
             set: order.status = Open
        "#,
    )
    .unwrap();

    let document = parse_rif_file(&root_path).unwrap();
    assert!(
        document
            .intents
            .iter()
            .any(|intent| intent.name == "Shared.ArchiveOrder")
    );
    assert!(
        document
            .intents
            .iter()
            .any(|intent| intent.name == "Ops.ArchiveOrder")
    );
    assert!(document.application.things.contains_key("Shared.Order"));
    assert!(
        document
            .application
            .collections
            .contains_key("Shared.orders")
    );
    assert_eq!(build_program(&document).modules[0].name, "Orders.Main");
    let rendered = render_rif_document(&document);
    let reparsed = parse_rif_text(&rendered).unwrap();
    let mut normalized_document = document.clone();
    normalized_document.source_path = None;
    assert_eq!(reparsed, normalized_document);

    let _ = fs::remove_file(imported_path);
    let _ = fs::remove_file(root_path);
}

#[test]
fn rif_import_aliases_endpoint_error_response_field_types() {
    let imported_path = std::env::temp_dir().join("eigl-endpoint-error-lib.rif.md");
    let root_path = std::env::temp_dir().join("eigl-endpoint-error-root.rif.md");
    fs::write(
        &imported_path,
        r#"
        app ErrorLibrary

        things:
          thing ErrorPayload
            field code: Text

        endpoints:
          endpoint GET /shared -> ShowShared
            error:
              payload: ErrorPayload
              payload = error_payload
            error NotFound:
              payload: ErrorPayload
              payload = error_payload

        intent ShowShared

        subject:
          error_payload: ErrorPayload
        "#,
    )
    .unwrap();
    fs::write(
        &root_path,
        r#"
        imports:
          import eigl-endpoint-error-lib.rif.md as Shared

        app Root

        intent RootIntent
        "#,
    )
    .unwrap();

    let document = parse_rif_file(&root_path).unwrap();
    let endpoint = document
        .application
        .endpoints
        .iter()
        .find(|endpoint| endpoint.path == "/Shared/shared")
        .unwrap();

    assert_eq!(endpoint.error_fields["payload"], "Shared.ErrorPayload");
    assert_eq!(
        endpoint.error_cases["NotFound"].response_fields["payload"],
        "Shared.ErrorPayload"
    );
    assert_eq!(check_document(&document), Vec::new());

    let _ = fs::remove_file(imported_path);
    let _ = fs::remove_file(root_path);
}

#[test]
fn rif_imports_only_exported_declarations_when_exports_are_present() {
    let imported_path = std::env::temp_dir().join("eigl-exported-order-lib.rif.md");
    let root_path = std::env::temp_dir().join("eigl-exported-order-root.rif.md");
    fs::write(
        &imported_path,
        r#"
        app OrderLibrary

        exports:
          export thing Order
          export collection orders
          export intent ArchiveOrder

        things:
          thing Order
            field status: State<Open, Archived>
          thing AuditLog
            field message: Text

        collections:
          collection orders: Order
          collection audit_logs: AuditLog

        intent ArchiveOrder

        subject:
          order: Order

        steps:
          1. Archive order
             set: order.status = Archived

        intent WriteAuditLog

        subject:
          audit: AuditLog

        steps:
          1. Write audit log
             set: audit.message = archived
        "#,
    )
    .unwrap();
    fs::write(
        &root_path,
        r#"
        imports:
          import eigl-exported-order-lib.rif.md as Shared

        app Orders

        things:
          thing Order
            field status: State<Open, Archived>

        intent ProcessOrder

        subject:
          order: Order

        steps:
          1. Open order
             set: order.status = Open
        "#,
    )
    .unwrap();

    let document = parse_rif_file(&root_path).unwrap();
    assert!(document.application.things.contains_key("Shared.Order"));
    assert!(!document.application.things.contains_key("Shared.AuditLog"));
    assert!(
        document
            .application
            .collections
            .contains_key("Shared.orders")
    );
    assert!(
        !document
            .application
            .collections
            .contains_key("Shared.audit_logs")
    );
    assert!(
        document
            .intents
            .iter()
            .any(|intent| intent.name == "Shared.ArchiveOrder")
    );
    assert!(
        !document
            .intents
            .iter()
            .any(|intent| intent.name == "Shared.WriteAuditLog")
    );

    let rendered = render_rif_document(&document);
    let reparsed = parse_rif_text(&rendered).unwrap();
    let mut normalized_document = document.clone();
    normalized_document.source_path = None;
    assert_eq!(reparsed, normalized_document);

    let _ = fs::remove_file(imported_path);
    let _ = fs::remove_file(root_path);
}

#[test]
fn checker_reports_unknown_exported_declarations() {
    let document = parse_rif_text(
        r#"
        app ExportedOrders

        exports:
          export intent MissingIntent

        intent ProcessOrder

        steps:
          1. Start
             set: report.count = 1
        "#,
    )
    .unwrap();

    let diagnostics = check_document(&document);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "EIGL_UNKNOWN_EXPORT")
    );
}

#[test]
fn checker_reports_unknown_endpoint_response_sources() {
    let document = parse_rif_text(
        r#"
        app ResponseApi

        things:
          thing Ticket
            field id: Text

        endpoints:
          endpoint POST /tickets -> OpenTicket
            respond:
              id = missing.id

        intent OpenTicket

        subject:
          ticket: Ticket

        steps:
          1. Open ticket
             set: ticket.id = T1
        "#,
    )
    .unwrap();

    let diagnostics = check_document(&document);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "EIGL_UNKNOWN_ENDPOINT_RESPONSE_SOURCE")
    );
}

#[test]
fn checker_reports_unknown_endpoint_requirement_references() {
    let document = parse_rif_text(
        r#"
        app ResponseApi

        things:
          thing Ticket
            field id: Text
            field status: State<Open, Closed>

        endpoints:
          endpoint GET /tickets/{id} -> ShowTicket
            requires:
              ticket.missing is Open
            bind:
              ticket.id = id

        intent ShowTicket

        subject:
          ticket: Ticket

        steps:
          1. Show ticket
             set: ticket.status = Open
        "#,
    )
    .unwrap();

    let diagnostics = check_document(&document);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "EIGL_UNKNOWN_ENDPOINT_REQUIREMENT_REFERENCE")
    );
}

#[test]
fn checker_rejects_step_outputs_in_endpoint_requirements() {
    let document = parse_rif_text(
        r#"
        app EndpointRequirementOutput

        things:
          thing Ticket
            field id: Text

        operations:
          operation Catalog.title(id: Text) -> Text

        endpoints:
          endpoint GET /ticket -> FetchTicketTitle
            requires:
              fetched_title exists

        intent FetchTicketTitle

        subject:
          ticket: Ticket

        steps:
          1. Fetch title
             call: Catalog.title(ticket.id)
             output: fetched_title: Text
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_OUTPUT_UNAVAILABLE".to_string()));
}

#[test]
fn checker_rejects_local_computes_in_endpoint_requirements() {
    let document = parse_rif_text(
        r#"
        app EndpointRequirementLocal

        things:
          thing Invoice
            field subtotal: Int
            field tax: Int

        endpoints:
          endpoint GET /invoice -> CalculateInvoice
            requires:
              line_total > 0

        intent CalculateInvoice

        subject:
          invoice: Invoice

        steps:
          1. Calculate total
             compute: line_total = invoice.subtotal + invoice.tax
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_LOCAL_VALUE_UNAVAILABLE".to_string()));
}

#[test]
fn checker_accepts_collection_keys_json_response_sources() {
    let document = parse_rif_file(fixture("ticket_api_app.rif.md")).unwrap();

    assert_eq!(check_document(&document), Vec::new());
}

#[test]
fn endpoint_request_schema_round_trips_and_checks() {
    let document = parse_rif_text(
        r#"
        app RequestApi

        things:
          thing Ticket
            field title: Text
            field estimate: Int

        endpoints:
          endpoint POST /tickets -> CreateTicket
            request:
              title: Text
              estimate: Int
            bind:
              ticket.title = title
              ticket.estimate = estimate

        intent CreateTicket

        subject:
          ticket: Ticket
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());
    let rendered = render_rif_document(&document);
    assert!(rendered.contains("    request:\n      estimate: Int\n      title: Text\n"));
    let reparsed = parse_rif_text(&rendered).unwrap();
    assert_eq!(reparsed, document);
}

#[test]
fn endpoint_request_binding_expressions_check_and_dispatch() {
    let binary = env!("CARGO_BIN_EXE_eigl");
    let source = temp_path("endpoint-binding-expression").with_extension("rif.md");
    let rif = r#"
        app RequestExpressionApi

        things:
          thing Ticket
            field estimate: Int

        endpoints:
          endpoint POST /tickets -> CreateTicket
            request:
              subtotal: Int
              tax: Int
            bind:
              ticket.estimate = subtotal + tax
            respond:
              estimate: Int
              estimate = ticket.estimate

        intent CreateTicket

        subject:
          ticket: Ticket
        "#;
    let document = parse_rif_text(rif).unwrap();

    assert_eq!(check_document(&document), Vec::new());

    fs::write(&source, rif).unwrap();
    let output = Command::new(binary)
        .args([
            "dispatch",
            source.to_str().unwrap(),
            "POST",
            "/tickets",
            "subtotal=19",
            "tax=4",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("ticket.estimate=23"));
    assert!(stdout.contains("response.estimate=23"));

    let _ = fs::remove_file(source);
}

#[test]
fn endpoint_query_parameters_check_dispatch_and_respond() {
    let binary = env!("CARGO_BIN_EXE_eigl");
    let source = temp_path("query-endpoint").with_extension("rif.md");
    let rif = r#"
        app QueryApi

        things:
          thing Report
            field status: State<Open, Closed>

        endpoints:
          endpoint GET /tickets -> FilterTickets
            request:
              query.status: State<Open, Closed>
            requires:
              query.status is Open else BadFilter
            bind:
              report.status = query.status
            respond:
              selected: State<Open, Closed>
              echo: State<Open, Closed>
              selected = report.status
              echo = query.status
            error BadFilter:
              status: 400 Bad Request
              code: Text
              code = failure

        intent FilterTickets

        subject:
          report: Report
        "#;
    let document = parse_rif_text(rif).unwrap();

    assert_eq!(check_document(&document), Vec::new());

    fs::write(&source, rif).unwrap();
    let output = Command::new(binary)
        .args([
            "dispatch",
            source.to_str().unwrap(),
            "GET",
            "/tickets?status=Open",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("dispatch succeeded"));
    assert!(stdout.contains("report.status=Open"));
    assert!(stdout.contains("response.selected=Open"));
    assert!(stdout.contains("response.echo=Open"));

    let rejected = Command::new(binary)
        .args([
            "dispatch",
            source.to_str().unwrap(),
            "GET",
            "/tickets?status=Closed",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();

    assert!(!rejected.status.success());
    let rejected_stdout = String::from_utf8_lossy(&rejected.stdout);
    assert!(rejected_stdout.contains("failure=BadFilter"));
    assert!(rejected_stdout.contains("error.code=BadFilter"));
    assert!(rejected_stdout.contains("http_status=400 Bad Request"));

    let _ = fs::remove_file(source);
}

#[test]
fn endpoint_responses_resolve_map_lookup_expressions() {
    let binary = env!("CARGO_BIN_EXE_eigl");
    let source = temp_path("map-lookup-endpoint").with_extension("rif.md");
    let state_in = temp_path("map-lookup-endpoint-state");
    let rif = r#"
        app DashboardApi

        things:
          thing Dashboard
            field counts: Map<Text, Int>
            field status: Text

        endpoints:
          endpoint GET /dashboard -> ShowDashboard
            respond:
              open: Int
              selected: Int
              open = dashboard.counts["open"]
              selected = dashboard.counts[dashboard.status]

        intent ShowDashboard

        subject:
          dashboard: Dashboard
        "#;
    let document = parse_rif_text(rif).unwrap();

    assert_eq!(check_document(&document), Vec::new());

    fs::write(&source, rif).unwrap();
    fs::write(
        &state_in,
        "dashboard.counts={\"open\":1,\"closed\":2}\ndashboard.status=closed\n",
    )
    .unwrap();
    let output = Command::new(binary)
        .args([
            "dispatch",
            source.to_str().unwrap(),
            "GET",
            "/dashboard",
            "--state-in",
            state_in.to_str().unwrap(),
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("dispatch succeeded"));
    assert!(stdout.contains("response.open=1"));
    assert!(stdout.contains("response.selected=2"));

    let _ = fs::remove_file(source);
    let _ = fs::remove_file(state_in);
}

#[test]
fn checker_reports_unknown_declared_endpoint_request_type() {
    let document = parse_rif_text(
        r#"
        app RequestApi

        things:
          thing Ticket
            field title: Text

        endpoints:
          endpoint POST /tickets -> CreateTicket
            request:
              title: UnknownType
            bind:
              ticket.title = title

        intent CreateTicket

        subject:
          ticket: Ticket
        "#,
    )
    .unwrap();

    assert!(
        check_document(&document)
            .iter()
            .any(|diagnostic| diagnostic.code == "EIGL_UNKNOWN_ENDPOINT_REQUEST_TYPE")
    );
}

#[test]
fn checker_reports_undeclared_endpoint_request_binding_sources() {
    let document = parse_rif_text(
        r#"
        app RequestApi

        things:
          thing Ticket
            field title: Text

        endpoints:
          endpoint POST /tickets -> CreateTicket
            request:
              title: Text
            bind:
              ticket.title = tilte

        intent CreateTicket

        subject:
          ticket: Ticket
        "#,
    )
    .unwrap();

    assert!(
        check_document(&document)
            .iter()
            .any(|diagnostic| diagnostic.code == "EIGL_UNKNOWN_ENDPOINT_BINDING_SOURCE")
    );
}

#[test]
fn checker_rejects_step_outputs_in_endpoint_bindings() {
    let document = parse_rif_text(
        r#"
        app EndpointBindingOutput

        things:
          thing Ticket
            field id: Text
            field title: Text

        operations:
          operation Catalog.title(id: Text) -> Text

        endpoints:
          endpoint POST /ticket -> FetchTicketTitle
            bind:
              ticket.title = fetched_title

        intent FetchTicketTitle

        subject:
          ticket: Ticket

        steps:
          1. Fetch title
             call: Catalog.title(ticket.id)
             output: fetched_title: Text
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_OUTPUT_UNAVAILABLE".to_string()));
}

#[test]
fn checker_rejects_local_computes_in_endpoint_bindings() {
    let document = parse_rif_text(
        r#"
        app EndpointBindingLocal

        things:
          thing Invoice
            field subtotal: Int
            field tax: Int
            field total: Int

        endpoints:
          endpoint POST /invoice -> CalculateInvoice
            bind:
              invoice.total = line_total

        intent CalculateInvoice

        subject:
          invoice: Invoice

        steps:
          1. Calculate total
             compute: line_total = invoice.subtotal + invoice.tax
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_LOCAL_VALUE_UNAVAILABLE".to_string()));
}

#[test]
fn checker_reports_endpoint_request_binding_type_mismatches() {
    let document = parse_rif_text(
        r#"
        app RequestApi

        things:
          thing Ticket
            field estimate: Int

        endpoints:
          endpoint POST /tickets -> CreateTicket
            request:
              estimate: Text
            bind:
              ticket.estimate = estimate

        intent CreateTicket

        subject:
          ticket: Ticket
        "#,
    )
    .unwrap();

    assert!(
        check_document(&document)
            .iter()
            .any(|diagnostic| diagnostic.code == "EIGL_ENDPOINT_BINDING_TYPE_MISMATCH")
    );
}

#[test]
fn checker_rejects_duplicate_endpoint_routes() {
    let document = parse_rif_text(
        r#"
        app DuplicateRoutes

        endpoints:
          endpoint GET /tickets -> ListTickets
          endpoint get /tickets -> SearchTickets

        intent ListTickets

        intent SearchTickets
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_DUPLICATE_ENDPOINT".to_string()));
}

#[test]
fn trigger_payload_schema_round_trips_and_checks() {
    let document = parse_rif_text(
        r#"
        app EventApi

        things:
          thing Job
            field id: Text
            field run_index: Int

        triggers:
          trigger nightly.cleanup -> RunCleanup
            payload:
              job_id: Text
              run_index: Int
            bind:
              job.id = event.job_id
              job.run_index = event.run_index

        intent RunCleanup

        subject:
          job: Job
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());
    let rendered = render_rif_document(&document);
    assert!(rendered.contains("    payload:\n      job_id: Text\n      run_index: Int\n"));
    let reparsed = parse_rif_text(&rendered).unwrap();
    assert_eq!(reparsed, document);
}

#[test]
fn checker_rejects_duplicate_trigger_names() {
    let document = parse_rif_text(
        r#"
        app DuplicateTriggers

        triggers:
          trigger nightly.cleanup -> RunCleanup
          trigger nightly.cleanup -> ArchiveCleanup

        intent RunCleanup

        intent ArchiveCleanup
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_DUPLICATE_TRIGGER".to_string()));
}

#[test]
fn trigger_payload_binding_expressions_check_and_emit() {
    let binary = env!("CARGO_BIN_EXE_eigl");
    let source = temp_path("trigger-binding-expression").with_extension("rif.md");
    let rif = r#"
        app TriggerExpressionApi

        things:
          thing Job
            field runs: Int

        triggers:
          trigger nightly.cleanup -> RunCleanup
            payload:
              subtotal: Int
              tax: Int
            bind:
              job.runs = event.subtotal + event.tax

        intent RunCleanup

        subject:
          job: Job
        "#;
    let document = parse_rif_text(rif).unwrap();

    assert_eq!(check_document(&document), Vec::new());

    fs::write(&source, rif).unwrap();
    let output = Command::new(binary)
        .args([
            "emit",
            source.to_str().unwrap(),
            "nightly.cleanup",
            "subtotal=19",
            "tax=4",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("emit succeeded"));
    assert!(stdout.contains("job.runs=23"));

    let _ = fs::remove_file(source);
}

#[test]
fn checker_accepts_trigger_text_metadata_literals_in_requirements() {
    let document = parse_rif_text(
        r#"
        app TriggerMetadataRequirements

        things:
          thing Job
            field id: Text

        triggers:
          trigger nightly.cleanup -> RunCleanup
            schedule: PT24H
            requires:
              event.name is nightly.cleanup
              event.schedule is PT24H
            bind:
              job.id = event.name

        intent RunCleanup

        subject:
          job: Job
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());
}

#[test]
fn checker_reports_unknown_declared_trigger_payload_type() {
    let document = parse_rif_text(
        r#"
        app EventApi

        things:
          thing Job
            field id: Text

        triggers:
          trigger nightly.cleanup -> RunCleanup
            payload:
              job_id: UnknownType
            bind:
              job.id = event.job_id

        intent RunCleanup

        subject:
          job: Job
        "#,
    )
    .unwrap();

    assert!(
        check_document(&document)
            .iter()
            .any(|diagnostic| diagnostic.code == "EIGL_UNKNOWN_TRIGGER_PAYLOAD_TYPE")
    );
}

#[test]
fn checker_reports_undeclared_trigger_payload_binding_sources() {
    let document = parse_rif_text(
        r#"
        app EventApi

        things:
          thing Job
            field id: Text

        triggers:
          trigger nightly.cleanup -> RunCleanup
            payload:
              job_id: Text
            bind:
              job.id = event.jbo_id

        intent RunCleanup

        subject:
          job: Job
        "#,
    )
    .unwrap();

    assert!(
        check_document(&document)
            .iter()
            .any(|diagnostic| diagnostic.code == "EIGL_UNKNOWN_TRIGGER_BINDING_SOURCE")
    );
}

#[test]
fn checker_rejects_step_outputs_in_trigger_bindings() {
    let document = parse_rif_text(
        r#"
        app TriggerBindingOutput

        things:
          thing Job
            field id: Text
            field title: Text

        operations:
          operation Catalog.title(id: Text) -> Text

        triggers:
          trigger nightly.cleanup -> RunCleanup
            bind:
              job.title = fetched_title

        intent RunCleanup

        subject:
          job: Job

        steps:
          1. Fetch title
             call: Catalog.title(job.id)
             output: fetched_title: Text
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_OUTPUT_UNAVAILABLE".to_string()));
}

#[test]
fn checker_rejects_local_computes_in_trigger_bindings() {
    let document = parse_rif_text(
        r#"
        app TriggerBindingLocal

        things:
          thing Job
            field subtotal: Int
            field tax: Int
            field total: Int

        triggers:
          trigger nightly.cleanup -> RunCleanup
            bind:
              job.total = line_total

        intent RunCleanup

        subject:
          job: Job

        steps:
          1. Calculate total
             compute: line_total = job.subtotal + job.tax
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_LOCAL_VALUE_UNAVAILABLE".to_string()));
}

#[test]
fn checker_rejects_step_outputs_in_trigger_requirements() {
    let document = parse_rif_text(
        r#"
        app TriggerRequirementOutput

        things:
          thing Job
            field id: Text

        operations:
          operation Catalog.title(id: Text) -> Text

        triggers:
          trigger nightly.cleanup -> RunCleanup
            requires:
              fetched_title exists

        intent RunCleanup

        subject:
          job: Job

        steps:
          1. Fetch title
             call: Catalog.title(job.id)
             output: fetched_title: Text
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_OUTPUT_UNAVAILABLE".to_string()));
}

#[test]
fn checker_rejects_local_computes_in_trigger_requirements() {
    let document = parse_rif_text(
        r#"
        app TriggerRequirementLocal

        things:
          thing Job
            field subtotal: Int
            field tax: Int

        triggers:
          trigger nightly.cleanup -> RunCleanup
            requires:
              line_total > 0

        intent RunCleanup

        subject:
          job: Job

        steps:
          1. Calculate total
             compute: line_total = job.subtotal + job.tax
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_LOCAL_VALUE_UNAVAILABLE".to_string()));
}

#[test]
fn checker_reports_trigger_payload_binding_type_mismatches() {
    let document = parse_rif_text(
        r#"
        app EventApi

        things:
          thing Job
            field run_index: Int

        triggers:
          trigger nightly.cleanup -> RunCleanup
            payload:
              run_index: Text
            bind:
              job.run_index = event.run_index

        intent RunCleanup

        subject:
          job: Job
        "#,
    )
    .unwrap();

    assert!(
        check_document(&document)
            .iter()
            .any(|diagnostic| diagnostic.code == "EIGL_TRIGGER_BINDING_TYPE_MISMATCH")
    );
}

#[test]
fn endpoint_response_schema_round_trips_and_checks() {
    let document = parse_rif_text(
        r#"
        app ResponseApi

        things:
          thing Ticket
            field title: Text
            field estimate: Int

        endpoints:
          endpoint GET /tickets/{id} -> ShowTicket
            respond:
              title: Text
              estimate: Int
              title = ticket.title
              estimate = ticket.estimate

        intent ShowTicket

        subject:
          ticket: Ticket
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());
    let rendered = render_rif_document(&document);
    assert!(rendered.contains("    respond:\n      estimate: Int\n      title: Text\n"));
    let reparsed = parse_rif_text(&rendered).unwrap();
    assert_eq!(reparsed, document);
}

#[test]
fn cli_dispatch_evaluates_literal_endpoint_responses() {
    let binary = env!("CARGO_BIN_EXE_eigl");
    let source = temp_path("literal-endpoint-response").with_extension("rif.md");
    fs::write(
        &source,
        r#"
        app HealthApi

        endpoints:
          endpoint GET /health -> Health
            respond:
              message: Text
              count: Int
              ok: Bool
              message = "ok"
              count = 3
              ok = true

        intent Health
        "#,
    )
    .unwrap();

    let output = Command::new(binary)
        .args(["dispatch", source.to_str().unwrap(), "GET", "/health"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("response.message=ok"));
    assert!(stdout.contains("response.count=3"));
    assert!(stdout.contains("response.ok=true"));

    let _ = fs::remove_file(source);
}

#[test]
fn cli_dispatch_maps_typed_object_return_aliases_into_endpoint_responses() {
    let binary = env!("CARGO_BIN_EXE_eigl");
    let source = temp_path("object-return-endpoint-response").with_extension("rif.md");
    let state_in = temp_path("object-return-endpoint-response-state");
    fs::write(
        &source,
        r#"
        app ObjectReturnEndpoint

        things:
          thing Ticket
            field id: Text
            field title: Text
            field estimate: Int

        endpoints:
          endpoint GET /ticket -> ShowTicket
            respond:
              ticket: Ticket
              ticket = result

        intent ShowTicket

        subject:
          ticket: Ticket

        returns:
          result: ticket
        "#,
    )
    .unwrap();
    fs::write(
        &state_in,
        "ticket.id=T-1\nticket.title=Printer\nticket.estimate=3\n",
    )
    .unwrap();

    let output = Command::new(binary)
        .args([
            "dispatch",
            source.to_str().unwrap(),
            "GET",
            "/ticket",
            "--state-in",
            state_in.to_str().unwrap(),
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(r#"response.ticket={"estimate":3,"id":"T-1","title":"Printer"}"#));

    let _ = fs::remove_file(source);
    let _ = fs::remove_file(state_in);
}

#[test]
fn checker_validates_endpoint_response_expression_types() {
    let document = parse_rif_text(
        r#"
        app BrokenResponseExpression

        things:
          thing Invoice
            field subtotal: Int
            field tax: Int

        endpoints:
          endpoint GET /invoice -> ShowInvoice
            respond:
              total: Text
              total = invoice.subtotal + invoice.tax

        intent ShowInvoice

        subject:
          invoice: Invoice
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_ENDPOINT_RESPONSE_TYPE_MISMATCH".to_string()));
}

#[test]
fn checker_validates_endpoint_response_return_alias_types() {
    let document = parse_rif_text(
        r#"
        app BrokenReturnAliasResponse

        things:
          thing Ticket
            field id: Text
            field title: Text

        endpoints:
          endpoint GET /ticket -> ShowTicket
            respond:
              ticket: Ticket
              ticket = result

        intent ShowTicket

        subject:
          ticket: Ticket

        returns:
          result: ticket.id
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_ENDPOINT_RESPONSE_TYPE_MISMATCH".to_string()));
}

#[test]
fn checker_rejects_unavailable_step_outputs_in_endpoint_responses() {
    let document = parse_rif_text(
        r#"
        app GuardedOutputEndpointResponse

        things:
          thing Ticket
            field id: Text

        operations:
          operation Catalog.title(id: Text) -> Text

        endpoints:
          endpoint GET /ticket -> FetchTicketTitle
            respond:
              title: Text
              title = fetched_title

        intent FetchTicketTitle

        subject:
          ticket: Ticket

        steps:
          1. Fetch title conditionally
             when: ticket.id == "T-1"
             call: Catalog.title(ticket.id)
             output: fetched_title: Text
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_OUTPUT_UNAVAILABLE".to_string()));
}

#[test]
fn checker_rejects_unavailable_local_computes_in_endpoint_responses() {
    let document = parse_rif_text(
        r#"
        app GuardedLocalEndpointResponse

        things:
          thing Invoice
            field subtotal: Int
            field tax: Int

        endpoints:
          endpoint GET /invoice -> CalculateInvoice
            respond:
              amount: Int
              amount = line_total

        intent CalculateInvoice

        subject:
          invoice: Invoice

        steps:
          1. Calculate total conditionally
             when: invoice.subtotal > 0
             compute: line_total = invoice.subtotal + invoice.tax
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_LOCAL_VALUE_UNAVAILABLE".to_string()));
}

#[test]
fn checker_reports_unknown_declared_endpoint_response_type() {
    let document = parse_rif_text(
        r#"
        app ResponseApi

        endpoints:
          endpoint GET /tickets -> ListTickets
            respond:
              total: UnknownType
              total = report.count

        intent ListTickets

        subject:
          report: Report
        "#,
    )
    .unwrap();

    assert!(
        check_document(&document)
            .iter()
            .any(|diagnostic| diagnostic.code == "EIGL_UNKNOWN_ENDPOINT_RESPONSE_TYPE")
    );
}

#[test]
fn checker_reports_unmapped_endpoint_response_fields() {
    let document = parse_rif_text(
        r#"
        app ResponseApi

        things:
          thing Report
            field count: Int

        endpoints:
          endpoint GET /tickets -> ListTickets
            respond:
              total: Int

        intent ListTickets

        subject:
          report: Report
        "#,
    )
    .unwrap();

    assert!(
        check_document(&document)
            .iter()
            .any(|diagnostic| diagnostic.code == "EIGL_ENDPOINT_RESPONSE_FIELD_UNMAPPED")
    );
}

#[test]
fn checker_reports_endpoint_response_type_mismatches() {
    let document = parse_rif_text(
        r#"
        app ResponseApi

        things:
          thing Report
            field count: Int

        endpoints:
          endpoint GET /tickets -> ListTickets
            respond:
              total: Text
              total = report.count

        intent ListTickets

        subject:
          report: Report
        "#,
    )
    .unwrap();

    assert!(
        check_document(&document)
            .iter()
            .any(|diagnostic| diagnostic.code == "EIGL_ENDPOINT_RESPONSE_TYPE_MISMATCH")
    );
}

#[test]
fn endpoint_error_response_schema_round_trips_and_checks() {
    let document = parse_rif_text(
        r#"
        app ErrorApi

        endpoints:
          endpoint GET /tickets/{id} -> ShowTicket
            error:
              status: 401 Unauthorized
              code: Text
              message: Text
              code = failure
              message = failure
            error NotFound:
              status: 404 Not Found
              code: Text
              message: Text
              code = failure
              message = "ticket not found"

        intent ShowTicket
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());
    let rendered = render_rif_document(&document);
    assert!(rendered.contains(
        "    error:\n      status: 401 Unauthorized\n      code: Text\n      message: Text\n"
    ));
    assert!(rendered.contains(
        "    error NotFound:\n      status: 404 Not Found\n      code: Text\n      message: Text\n"
    ));
    let reparsed = parse_rif_text(&rendered).unwrap();
    assert_eq!(reparsed, document);
}

#[test]
fn checker_reports_unknown_declared_endpoint_error_response_type() {
    let document = parse_rif_text(
        r#"
        app ErrorApi

        endpoints:
          endpoint GET /tickets -> ListTickets
            error:
              code: UnknownType
              code = failure

        intent ListTickets
        "#,
    )
    .unwrap();

    assert!(
        check_document(&document)
            .iter()
            .any(|diagnostic| diagnostic.code == "EIGL_UNKNOWN_ENDPOINT_RESPONSE_TYPE")
    );
}

#[test]
fn checker_reports_unmapped_endpoint_error_response_fields() {
    let document = parse_rif_text(
        r#"
        app ErrorApi

        endpoints:
          endpoint GET /tickets -> ListTickets
            error NotFound:
              code: Text

        intent ListTickets
        "#,
    )
    .unwrap();

    assert!(
        check_document(&document)
            .iter()
            .any(|diagnostic| diagnostic.code == "EIGL_ENDPOINT_RESPONSE_FIELD_UNMAPPED")
    );
}

#[test]
fn checker_reports_endpoint_error_response_type_mismatches() {
    let document = parse_rif_text(
        r#"
        app ErrorApi

        things:
          thing Report
            field count: Int

        endpoints:
          endpoint GET /tickets -> ListTickets
            error:
              code: Text
              code = report.count

        intent ListTickets

        subject:
          report: Report
        "#,
    )
    .unwrap();

    assert!(
        check_document(&document)
            .iter()
            .any(|diagnostic| diagnostic.code == "EIGL_ENDPOINT_RESPONSE_TYPE_MISMATCH")
    );
}

#[test]
fn checker_reports_invalid_endpoint_response_status() {
    let document = parse_rif_text(
        r#"
        app ResponseApi

        endpoints:
          endpoint POST /tickets -> OpenTicket
            respond:
              status: 700 Nope

        intent OpenTicket

        steps:
          1. Open ticket
             set: report.count = 1
        "#,
    )
    .unwrap();

    let diagnostics = check_document(&document);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "EIGL_INVALID_ENDPOINT_RESPONSE_STATUS")
    );
}

#[test]
fn checker_reports_invalid_endpoint_error_status() {
    let document = parse_rif_text(
        r#"
        app ResponseApi

        endpoints:
          endpoint POST /tickets -> OpenTicket
            error:
              status: 99 TooLow
              code = failure

        intent OpenTicket

        steps:
          1. Open ticket
             set: report.count = 1
        "#,
    )
    .unwrap();

    let diagnostics = check_document(&document);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "EIGL_INVALID_ENDPOINT_ERROR_STATUS")
    );
}

#[test]
fn parser_and_renderer_round_trip_exports_section() {
    let document = parse_rif_text(
        r#"
        app OrderLibrary

        exports:
          export thing Order
          export collection orders
          export intent ArchiveOrder

        things:
          thing Order
            field status: State<Open, Archived>

        collections:
          collection orders: Order

        intent ArchiveOrder

        subject:
          order: Order

        steps:
          1. Archive order
             set: order.status = Archived
        "#,
    )
    .unwrap();

    assert_eq!(document.application.exports.len(), 3);
    let rendered = render_rif_document(&document);
    assert!(rendered.contains("exports:"));
    assert!(rendered.contains("  export intent ArchiveOrder"));
    let reparsed = parse_rif_text(&rendered).unwrap();
    assert_eq!(reparsed, document);
}

#[test]
fn rif_imports_report_conflicting_declarations() {
    let first_path = std::env::temp_dir().join("eigl-ticket-shared-a.rif.md");
    let second_path = std::env::temp_dir().join("eigl-ticket-shared-b.rif.md");
    let root_path = std::env::temp_dir().join("eigl-ticket-conflict-root.rif.md");
    fs::write(
        &first_path,
        r#"
        app TicketApi

        things:
          thing Ticket
            field id: Text

        intent ArchiveTicket
        "#,
    )
    .unwrap();
    fs::write(
        &second_path,
        r#"
        app TicketApi

        things:
          thing Ticket
            field id: Text

        intent CloseTicket
        "#,
    )
    .unwrap();
    fs::write(
        &root_path,
        r#"
        imports:
          import eigl-ticket-shared-a.rif.md
          import eigl-ticket-shared-b.rif.md

        app TicketApi

        intent TicketDashboard
        "#,
    )
    .unwrap();

    let error = parse_rif_file(&root_path).unwrap_err();
    assert!(error.contains("conflicting imported thing 'Ticket'"));

    let _ = fs::remove_file(first_path);
    let _ = fs::remove_file(second_path);
    let _ = fs::remove_file(root_path);
}

#[test]
fn normalize_command_renders_canonical_rif_text() {
    let binary = env!("CARGO_BIN_EXE_eigl");
    let source = std::env::temp_dir().join("eigl-normalize-source.rif.md");
    fs::write(
        &source,
        r#"
        app NormalizeMe

        things:
          thing Ticket
            field id: Text
            field status: State<Open, Closed>

        collections:
          collection tickets: Ticket

        intent Close Ticket

        subject:
          ticket: Ticket

        requires:
          ticket.status is Open

        state transition:
          ticket.status: Open -> Closed

        steps:
          1. Close ticket
             set: ticket.status = Closed
        "#,
    )
    .unwrap();

    let output = Command::new(binary)
        .args(["normalize", source.to_str().unwrap()])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("intent Close Ticket"));
    assert!(stdout.contains("collections:"));
    assert!(stdout.contains("collection tickets: Ticket"));
    assert!(stdout.contains("steps:"));
    assert!(stdout.contains("set: ticket.status = Closed"));
}

#[test]
fn canonical_rif_round_trips_through_parser() {
    for fixture_name in [
        "ticket_api_app.rif.md",
        "ticket_event_app.rif.md",
        "branch_invoice_app.rif.md",
        "parallel_invoice_app.rif.md",
    ] {
        let source = fs::read_to_string(fixture(fixture_name)).unwrap();
        let document = parse_rif_text(&source).unwrap();
        let rendered = render_rif_document(&document);
        let reparsed = parse_rif_text(&rendered).unwrap();
        assert_eq!(reparsed, document, "round-trip changed {fixture_name}");
    }
}

#[test]
fn canonical_rif_round_trips_after_rsl_elaboration() {
    let source = fs::read_to_string(fixture("domain_sentence_app.rsl.md")).unwrap();
    let document = parse_rsl_text(&source).unwrap();
    let rendered = render_rif_document(&document);
    let reparsed = parse_rif_text(&rendered).unwrap();
    assert_eq!(reparsed, document);
}

#[test]
fn cli_llm_roundtrip_uses_completion_endpoint() {
    let binary = env!("CARGO_BIN_EXE_eigl");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let response_spec = fs::read_to_string(fixture("domain_sentence_app.rsl.md")).unwrap();
    let response_body = format!(
        "{{\"content\":{}}}",
        json_string(&format!(
            "<think>ignore</think>\n```rsl\n{response_spec}\n```"
        ))
    );

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        loop {
            let mut line = String::new();
            let read = reader.read_line(&mut line).unwrap();
            if read == 0 || line == "\r\n" {
                break;
            }
        }
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            response_body.len(),
            response_body
        );
        stream.write_all(response.as_bytes()).unwrap();
    });

    let output = Command::new(binary)
        .args([
            "llm-roundtrip",
            "examples/domain_sentence_app.rsl.md",
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/completion", addr.port()),
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();

    server.join().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("roundtrip=ok"));
    assert!(stdout.contains("intent Describe Domain"));
}

#[test]
fn cli_llm_roundtrip_uses_chat_completion_messages() {
    let binary = env!("CARGO_BIN_EXE_eigl");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let response_spec = fs::read_to_string(fixture("domain_sentence_app.rsl.md")).unwrap();
    let response_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(&response_spec)
    );

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut content_length = 0usize;
        loop {
            let mut line = String::new();
            let read = reader.read_line(&mut line).unwrap();
            if read == 0 || line == "\r\n" {
                break;
            }
            if let Some(value) = line
                .strip_prefix("Content-Length:")
                .or_else(|| line.strip_prefix("content-length:"))
            {
                content_length = value.trim().parse().unwrap();
            }
        }
        let mut request_body = vec![0; content_length];
        reader.read_exact(&mut request_body).unwrap();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            response_body.len(),
            response_body
        );
        stream.write_all(response.as_bytes()).unwrap();
        String::from_utf8(request_body).unwrap()
    });

    let output = Command::new(binary)
        .args([
            "llm-roundtrip",
            "examples/domain_sentence_app.rsl.md",
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();

    let request_body = server.join().unwrap();
    assert!(output.status.success());
    assert!(request_body.contains(r#""messages":"#));
    assert!(request_body.contains(r#""chat_template_kwargs":{"enable_thinking":false}"#));
    assert!(request_body.contains("controlled EIGL RSL"));
    assert!(request_body.contains("Do not output descriptive Markdown headings"));
    assert!(!request_body.contains(r#""prompt":"#));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("roundtrip=ok"));
}

#[test]
fn cli_patch_applies_intent_scoped_rif_edits() {
    let binary = env!("CARGO_BIN_EXE_eigl");
    let output = Command::new(binary)
        .args([
            "patch",
            "examples/confirm_order.rif.md",
            "examples/confirm_order.retry.patch",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("failure behavior:"));
    assert!(stdout.contains("if step \"Reserve inventory\" fails:"));
    assert!(stdout.contains("stop with InventoryUnavailable"));
    assert!(stdout.contains("guarantees:"));
    assert!(stdout.contains("if inventory reservation fails:"));
    assert!(stdout.contains("reservation is released"));
    let patched = parse_rif_text(&stdout).unwrap();
    assert_eq!(render_rif_document(&patched), stdout.trim());
}

#[test]
fn parser_and_checker_handle_unresolved_questions() {
    let source = fs::read_to_string(fixture("ambiguous_billing.rif.md")).unwrap();
    let document = parse_rif_text(&source).unwrap();

    assert_eq!(document.intent.unresolved_questions.len(), 1);
    assert!(
        document.intent.unresolved_questions[0]
            .text
            .contains("refund payment")
    );
    let rendered = render_rif_document(&document);
    let reparsed = parse_rif_text(&rendered).unwrap();
    assert_eq!(reparsed, document);
    let diagnostics = check_document(&document);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "EIGL_UNRESOLVED_QUESTIONS_PRESENT")
    );
}

#[test]
fn cli_patch_can_extend_application_domain_model() {
    let binary = env!("CARGO_BIN_EXE_eigl");
    let patch_path = std::env::temp_dir().join("eigl-ticket-domain.patch");
    fs::write(
        &patch_path,
        r#"
        patch Add ticket description field

        target:
          app TicketApi

        change:
          add field Ticket.description: Text
        "#,
    )
    .unwrap();

    let output = Command::new(binary)
        .args([
            "patch",
            "examples/ticket_api_app.rif.md",
            patch_path.to_str().unwrap(),
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("field description: Text"));
    let patched = parse_rif_text(&stdout).unwrap();
    assert_eq!(render_rif_document(&patched), stdout.trim());
}

#[test]
fn parser_reads_collections_section() {
    let document = parse_rif_text(
        r#"
        app TicketApi

        things:
          thing Ticket
            field id: Text
            field status: State<New, Open>

        collections:
          collection tickets: Ticket

        intent Open Ticket

        subject:
          ticket: Ticket
        "#,
    )
    .unwrap();

    assert_eq!(document.application.collections["tickets"].name, "tickets");
    assert_eq!(
        document.application.collections["tickets"].type_name,
        "Ticket"
    );
}

#[test]
fn parser_reads_unique_collection_constraints() {
    let document = parse_rif_file(fixture("profile_registry_app.rif.md")).unwrap();

    assert_eq!(
        document.application.collections["profiles"].unique_fields,
        vec!["email".to_string()]
    );
    assert_eq!(
        document.application.module.as_deref(),
        Some("Profiles.Registry")
    );
    let rendered = render_rif_document(&document);
    let reparsed = parse_rif_text(&rendered).unwrap();
    let mut normalized_document = document.clone();
    normalized_document.source_path = None;
    assert_eq!(reparsed, normalized_document);
    assert_eq!(
        build_program(&document).modules[0].name,
        "Profiles.Registry"
    );
    let program = build_program(&document);
    assert!(
        program
            .graph
            .find_node("module", "Profiles.Registry")
            .is_some()
    );
    assert!(
        program
            .graph
            .has_edge("contains", "Profiles.Registry", "Profile")
    );
}

#[test]
fn collection_queries_count_and_delete_filtered_records() {
    let binary = env!("CARGO_BIN_EXE_eigl");
    let data_in = temp_path("collections-in");
    let data_out = temp_path("collections-out");
    fs::write(
        &data_in,
        "tickets.T1.id=T1\ntickets.T1.status=Closed\ntickets.T1.title=Stale\ntickets.T2.id=T2\ntickets.T2.status=Open\ntickets.T2.title=Active\n",
    )
    .unwrap();

    let run = Command::new(binary)
        .args([
            "run",
            "examples/ticket_maintenance_app.rif.md",
            "--data-in",
            data_in.to_str().unwrap(),
            "--data-out",
            data_out.to_str().unwrap(),
            "report.closed_count=0",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(run.status.success());
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(stdout.contains("report.closed_count=1"));
    let saved = fs::read_to_string(&data_out).unwrap();
    assert!(!saved.contains("tickets.T1.status=Closed"));
    assert!(!saved.contains("tickets.T1.title=Stale"));
    assert!(saved.contains("tickets.T2.status=Open"));
    assert!(saved.contains("tickets.T2.title=Active"));

    let _ = fs::remove_file(data_in);
    let _ = fs::remove_file(data_out);
}

#[test]
fn collection_queries_render_matching_records_as_json_arrays() {
    let state = [
        ("tickets.T1.id".to_string(), "T1".to_string()),
        ("tickets.T1.status".to_string(), "Closed".to_string()),
        ("tickets.T1.title".to_string(), "Stale".to_string()),
        ("tickets.T2.id".to_string(), "T2".to_string()),
        ("tickets.T2.status".to_string(), "Open".to_string()),
        ("tickets.T2.title".to_string(), "Active".to_string()),
    ]
    .into();

    assert_eq!(
        collection_path_value(&state, "tickets[status=Closed].records_json").as_deref(),
        Some(r#"[{"id":"T1","status":"Closed","title":"Stale"}]"#)
    );
}

#[test]
fn collection_record_json_preserves_json_scalar_and_container_values() {
    let state = [
        ("tickets.T1.id".to_string(), "T1".to_string()),
        ("tickets.T1.open".to_string(), "true".to_string()),
        ("tickets.T1.count".to_string(), "2".to_string()),
        ("tickets.T1.optional".to_string(), "None".to_string()),
        (
            "tickets.T1.meta".to_string(),
            r#"{"kind":"ticket"}"#.to_string(),
        ),
        (
            "tickets.T1.tags".to_string(),
            r#"["printer","urgent"]"#.to_string(),
        ),
    ]
    .into();

    assert_eq!(
        collection_path_value(&state, "tickets.records_json").as_deref(),
        Some(
            r#"[{"count":2,"id":"T1","meta":{"kind":"ticket"},"open":true,"optional":null,"tags":["printer","urgent"]}]"#
        )
    );
}

#[test]
fn collection_record_json_returns_the_first_matching_record_object() {
    let state = [
        ("tickets.T1.id".to_string(), "T1".to_string()),
        ("tickets.T1.status".to_string(), "Closed".to_string()),
        ("tickets.T1.priority".to_string(), "3".to_string()),
        ("tickets.T2.id".to_string(), "T2".to_string()),
        ("tickets.T2.status".to_string(), "Open".to_string()),
        ("tickets.T2.priority".to_string(), "1".to_string()),
        ("request.status".to_string(), "Open".to_string()),
    ]
    .into();

    assert_eq!(
        collection_path_value(&state, "tickets[status=request.status].record_json").as_deref(),
        Some(r#"{"id":"T2","priority":1,"status":"Open"}"#)
    );
}

#[test]
fn endpoint_responses_return_typed_collection_records() {
    let binary = env!("CARGO_BIN_EXE_eigl");
    let source = temp_path("typed-collection-records").with_extension("rif.md");
    let data_in = temp_path("typed-collection-records-data");
    fs::write(
        &source,
        r#"
        app TypedCollectionRecords

        things:
          thing Ticket
            field id: Text
            field status: State<Open, Closed>
            field title: Text

        collections:
          collection tickets: Ticket

        endpoints:
          endpoint GET /tickets -> ListTickets
            respond:
              items: List<Ticket>
              items = tickets.records
          endpoint GET /tickets/{id} -> ShowTicket
            requires:
              tickets[id].id exists else NotFound
            respond:
              item: Ticket
              item = tickets[id].record

        intent ListTickets

        intent ShowTicket
        "#,
    )
    .unwrap();
    fs::write(
        &data_in,
        "tickets.T1.id=T1\ntickets.T1.status=Open\ntickets.T1.title=Printer\n",
    )
    .unwrap();

    let document = parse_rif_file(source.to_str().unwrap()).unwrap();
    let diagnostics = check_document(&document);
    assert!(
        diagnostics.is_empty(),
        "unexpected diagnostics: {diagnostics:?}"
    );

    let list_output = Command::new(binary)
        .args([
            "dispatch",
            source.to_str().unwrap(),
            "GET",
            "/tickets",
            "--data-in",
            data_in.to_str().unwrap(),
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(
        list_output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&list_output.stdout),
        String::from_utf8_lossy(&list_output.stderr)
    );
    let list_stdout = String::from_utf8_lossy(&list_output.stdout);
    assert!(
        list_stdout.contains(r#"response.items=[{"id":"T1","status":"Open","title":"Printer"}]"#)
    );

    let detail_output = Command::new(binary)
        .args([
            "dispatch",
            source.to_str().unwrap(),
            "GET",
            "/tickets/T1",
            "--data-in",
            data_in.to_str().unwrap(),
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(
        detail_output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&detail_output.stdout),
        String::from_utf8_lossy(&detail_output.stderr)
    );
    let detail_stdout = String::from_utf8_lossy(&detail_output.stdout);
    assert!(
        detail_stdout.contains(r#"response.item={"id":"T1","status":"Open","title":"Printer"}"#)
    );

    let _ = fs::remove_file(source);
    let _ = fs::remove_file(data_in);
}

#[test]
fn predicates_evaluate_exists_for_state_and_collection_paths() {
    let state = [
        ("ticket.id".to_string(), "T1".to_string()),
        ("tickets.T1.id".to_string(), "T1".to_string()),
    ]
    .into();

    assert!(predicate::evaluate("ticket.id exists", &state, &[].into()));
    assert!(predicate::evaluate(
        "tickets[ticket.id].id exists",
        &state,
        &[].into()
    ));
    assert!(!predicate::evaluate(
        "tickets.T2.id exists",
        &state,
        &[].into()
    ));
    assert!(predicate::evaluate(
        "not tickets.T2.id exists",
        &state,
        &[].into()
    ));
}

#[test]
fn predicates_compare_decimal_and_money_values() {
    let state = [
        ("invoice.subtotal".to_string(), "USD:20.00".to_string()),
        ("invoice.minimum".to_string(), "USD:10.00".to_string()),
        ("invoice.discount_rate".to_string(), "0.25".to_string()),
    ]
    .into();

    assert!(predicate::evaluate(
        "invoice.discount_rate < 0.50",
        &state,
        &[].into()
    ));
    assert!(predicate::evaluate(
        "invoice.subtotal >= invoice.minimum",
        &state,
        &[].into()
    ));
    assert!(!predicate::evaluate(
        "invoice.subtotal < EUR:20.00",
        &state,
        &[].into()
    ));
}

#[test]
fn predicates_compare_time_and_duration_values() {
    let state = [
        (
            "job.starts_at".to_string(),
            "2026-05-20T09:30:00Z".to_string(),
        ),
        (
            "job.deadline".to_string(),
            "2026-05-20T10:00:00Z".to_string(),
        ),
        ("job.timeout".to_string(), "PT30M".to_string()),
    ]
    .into();

    assert!(predicate::evaluate(
        "job.starts_at < job.deadline",
        &state,
        &[].into()
    ));
    assert!(predicate::evaluate(
        "job.timeout <= PT1H",
        &state,
        &[].into()
    ));
    assert!(predicate::evaluate("P1D == PT24H", &state, &[].into()));
    assert!(!predicate::evaluate("P1M < P2M", &state, &[].into()));
}

#[test]
fn unique_collection_constraints_reject_duplicate_records() {
    let binary = env!("CARGO_BIN_EXE_eigl");
    let state_in = temp_path("unique-collection-in");
    fs::write(
        &state_in,
        "profiles.P1.email=alice@example.com\nprofiles.P2.email=alice@example.com\nprofiles.P1.age=30\nprofiles.P2.age=22\nreport.count=0\n",
    )
    .unwrap();

    let run = Command::new(binary)
        .args([
            "run",
            "examples/profile_registry_app.rif.md",
            "--state-in",
            state_in.to_str().unwrap(),
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();

    assert!(!run.status.success());
    let stderr = String::from_utf8_lossy(&run.stderr);
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(stderr.contains("duplicate value") || stdout.contains("duplicate value"));

    let _ = fs::remove_file(state_in);
}

#[test]
fn simulation_iterates_over_filtered_collection_records() {
    let document = parse_rif_file(fixture("ticket_iteration_app.rif.md")).unwrap();
    assert_eq!(check_document(&document), Vec::new());

    let result = simulate(
        &document,
        [
            ("report.closed_count".to_string(), "0".to_string()),
            ("tickets.T1.id".to_string(), "T1".to_string()),
            ("tickets.T1.status".to_string(), "Closed".to_string()),
            ("tickets.T1.title".to_string(), "Stale".to_string()),
            ("tickets.T2.id".to_string(), "T2".to_string()),
            ("tickets.T2.status".to_string(), "Open".to_string()),
            ("tickets.T2.title".to_string(), "Active".to_string()),
        ]
        .into(),
        [].into(),
    );

    assert_eq!(result.status, "succeeded");
    assert_eq!(result.final_state["report.closed_count"], "1");
    assert!(!result.final_state.contains_key("tickets.T1.status"));
    assert!(!result.final_state.contains_key("tickets.T1.title"));
    assert!(result.final_state.contains_key("tickets.T2.status"));
}

#[test]
fn collection_record_projection_iteration_yields_typed_records() {
    let document = parse_rif_text(
        r#"
        app TicketReports

        things:
          thing Ticket
            field id: Text
            field status: State<Open, Closed>
            field title: Text

          thing Report
            field titles: List<Text>
            field last_id: Text

        collections:
          collection tickets: Ticket

        intent SummarizeOpenTickets

        subject:
          report: Report

        steps:
          1. Collect open ticket titles
             for each: tickets[status=Open].records as ticket
             append: report.titles += ticket.title
             set: report.last_id = ticket.id
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());

    let state: std::collections::BTreeMap<String, String> = [
        ("report.titles".to_string(), "[]".to_string()),
        ("report.last_id".to_string(), "".to_string()),
        ("tickets.T1.id".to_string(), "T1".to_string()),
        ("tickets.T1.status".to_string(), "Closed".to_string()),
        ("tickets.T1.title".to_string(), "Stale".to_string()),
        ("tickets.T2.id".to_string(), "T2".to_string()),
        ("tickets.T2.status".to_string(), "Open".to_string()),
        ("tickets.T2.title".to_string(), "Active".to_string()),
        ("tickets.T3.id".to_string(), "T3".to_string()),
        ("tickets.T3.status".to_string(), "Open".to_string()),
        ("tickets.T3.title".to_string(), "Review".to_string()),
    ]
    .into();

    let result = run_bytecode(
        &lower_document(&document),
        state.clone(),
        Default::default(),
    );
    assert_eq!(result.status, "succeeded");
    assert_eq!(result.final_state["report.titles"], "[Active,Review]");
    assert_eq!(result.final_state["report.last_id"], "T3");

    let simulation = simulate(&document, state, Default::default());
    assert_eq!(simulation.status, "succeeded");
    assert_eq!(simulation.final_state["report.titles"], "[Active,Review]");
    assert_eq!(simulation.final_state["report.last_id"], "T3");
}

#[test]
fn set_steps_upsert_typed_objects_into_collections() {
    let document = parse_rif_text(
        r#"
        app CollectionObjectUpsert

        things:
          thing Ticket
            field id: Text
            field title: Text
            field status: State<New, Open>

        collections:
          collection tickets: Ticket

        intent StoreTicket

        subject:
          ticket: Ticket

        steps:
          1. Store ticket record
             set: ticket.status = Open
             set: tickets[ticket.id] = ticket
             changes: tickets
        "#,
    )
    .unwrap();
    assert_eq!(check_document(&document), Vec::new());

    let state: std::collections::BTreeMap<String, String> = [
        ("ticket.id".to_string(), "T-1".to_string()),
        ("ticket.title".to_string(), "Printer".to_string()),
        ("ticket.status".to_string(), "New".to_string()),
        ("tickets.T-1.title".to_string(), "Old title".to_string()),
        ("tickets.T-1.status".to_string(), "New".to_string()),
    ]
    .into();

    let bytecode = run_bytecode(&lower_document(&document), state.clone(), [].into());
    assert_eq!(bytecode.status, "succeeded");
    assert_eq!(bytecode.final_state["tickets.T-1.id"], "T-1");
    assert_eq!(bytecode.final_state["tickets.T-1.title"], "Printer");
    assert_eq!(bytecode.final_state["tickets.T-1.status"], "Open");

    let simulated = simulate(&document, state, [].into());
    assert_eq!(simulated.status, "succeeded");
    assert_eq!(simulated.final_state["tickets.T-1.id"], "T-1");
    assert_eq!(simulated.final_state["tickets.T-1.title"], "Printer");
    assert_eq!(simulated.final_state["tickets.T-1.status"], "Open");
}

#[test]
fn collection_queries_participate_in_requirements() {
    let document = parse_rif_text(
        r#"
        app TicketMetrics

        things:
          thing Ticket
            field id: Text
            field status: State<Open, Closed>

        collections:
          collection tickets: Ticket

        intent CountClosedTickets

        requires:
          tickets[status=Closed].count is 1

        subject:
          report: Report

        things:
          thing Report
            field closed_count: Int

        steps:
          1. Count closed tickets
             set: report.closed_count = tickets[status=Closed].count
        "#,
    )
    .unwrap();

    let state = [
        ("report.closed_count".to_string(), "0".to_string()),
        ("tickets.T1.id".to_string(), "T1".to_string()),
        ("tickets.T1.status".to_string(), "Closed".to_string()),
        ("tickets.T2.id".to_string(), "T2".to_string()),
        ("tickets.T2.status".to_string(), "Open".to_string()),
    ]
    .into();

    assert!(check_document(&document).is_empty());
    let result = simulate(&document, state, [].into());
    assert_eq!(result.status, "succeeded");
    assert_eq!(result.final_state["report.closed_count"], "1");
}

#[test]
fn collection_selectors_resolve_live_state_values() {
    let document = parse_rif_text(
        r#"
        app TicketLookup

        things:
          thing Ticket
            field id: Text
            field status: State<Open, Closed>

          thing Report
            field ticket_id: Text
            field selected_status: State<Open, Closed>

        collections:
          collection tickets: Ticket

        intent SelectTicket

        requires:
          tickets[report.ticket_id].status is Closed

        subject:
          report: Report

        steps:
          1. Capture selected status
             set: report.selected_status = tickets[report.ticket_id].status
        "#,
    )
    .unwrap();

    let state: std::collections::BTreeMap<String, String> = [
        ("report.ticket_id".to_string(), "T1".to_string()),
        ("report.selected_status".to_string(), "Open".to_string()),
        ("tickets.T1.id".to_string(), "T1".to_string()),
        ("tickets.T1.status".to_string(), "Closed".to_string()),
        ("tickets.T2.id".to_string(), "T2".to_string()),
        ("tickets.T2.status".to_string(), "Open".to_string()),
    ]
    .into();

    assert!(check_document(&document).is_empty());
    let result = simulate(&document, state.clone(), [].into());
    assert_eq!(result.status, "succeeded");
    assert_eq!(result.final_state["report.selected_status"], "Closed");

    let bytecode = lower_document(&document);
    let result = run_bytecode(&bytecode, state, [].into());
    assert_eq!(result.status, "succeeded");
    assert_eq!(result.final_state["report.selected_status"], "Closed");
}

#[test]
fn collection_field_selectors_resolve_live_expected_values() {
    let document = parse_rif_text(
        r#"
        app TicketLookup

        things:
          thing Ticket
            field id: Text
            field status: State<Open, Closed>

          thing Report
            field desired_status: State<Open, Closed>
            field selected_count: Int

        collections:
          collection tickets: Ticket

        intent CountSelectedTickets

        requires:
          tickets[status=report.desired_status].count is 1

        subject:
          report: Report

        steps:
          1. Count selected tickets
             set: report.selected_count = tickets[status=report.desired_status].count
        "#,
    )
    .unwrap();

    let state: std::collections::BTreeMap<String, String> = [
        ("report.desired_status".to_string(), "Closed".to_string()),
        ("report.selected_count".to_string(), "0".to_string()),
        ("tickets.T1.id".to_string(), "T1".to_string()),
        ("tickets.T1.status".to_string(), "Closed".to_string()),
        ("tickets.T2.id".to_string(), "T2".to_string()),
        ("tickets.T2.status".to_string(), "Open".to_string()),
    ]
    .into();

    assert!(check_document(&document).is_empty());
    let result = simulate(&document, state.clone(), [].into());
    assert_eq!(result.status, "succeeded");
    assert_eq!(result.final_state["report.selected_count"], "1");

    let bytecode = lower_document(&document);
    let result = run_bytecode(&bytecode, state, [].into());
    assert_eq!(result.status, "succeeded");
    assert_eq!(result.final_state["report.selected_count"], "1");
}

#[test]
fn collection_comparison_filters_drive_requirements_and_runtime_flow() {
    let document = parse_rif_text(
        r#"
        app TicketPriorityMetrics

        things:
          thing Ticket
            field id: Text
            field priority: Int

          thing Report
            field high_priority_count: Int

        collections:
          collection tickets: Ticket

        intent CountHighPriorityTickets

        requires:
          tickets[priority>=2].count is 2

        subject:
          report: Report

        steps:
          1. Count high priority tickets
             set: report.high_priority_count = tickets[priority>=2].count
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());

    let state: std::collections::BTreeMap<String, String> = [
        ("report.high_priority_count".to_string(), "0".to_string()),
        ("tickets.T1.id".to_string(), "T1".to_string()),
        ("tickets.T1.priority".to_string(), "1".to_string()),
        ("tickets.T2.id".to_string(), "T2".to_string()),
        ("tickets.T2.priority".to_string(), "2".to_string()),
        ("tickets.T3.id".to_string(), "T3".to_string()),
        ("tickets.T3.priority".to_string(), "3".to_string()),
    ]
    .into();

    let simulated = simulate(&document, state.clone(), [].into());
    assert_eq!(simulated.status, "succeeded");
    assert_eq!(simulated.final_state["report.high_priority_count"], "2");

    let bytecode = run_bytecode(&lower_document(&document), state, [].into());
    assert_eq!(bytecode.status, "succeeded");
    assert_eq!(bytecode.final_state["report.high_priority_count"], "2");
}

#[test]
fn collection_contains_filters_drive_requirements_and_runtime_flow() {
    let document = parse_rif_text(
        r#"
        app TicketContainmentMetrics

        things:
          thing Ticket
            field id: Text
            field title: Text
            field tags: List<Text>
            field flags: Map<Text, Bool>

          thing Report
            field printer_count: Int
            field urgent_count: Int
            field vip_count: Int
            field urgent_printer_count: Int

        collections:
          collection tickets: Ticket

        intent CountContainedTickets

        requires:
          tickets[title contains "printer"].count is 2
          tickets[tags contains "urgent"].count is 2
          tickets[flags contains "vip"].count is 1
          tickets[title contains "printer" and tags contains "urgent"].count is 1

        subject:
          report: Report

        steps:
          1. Count matching tickets
             set: report.printer_count = tickets[title contains "printer"].count
             set: report.urgent_count = tickets[tags contains "urgent"].count
             set: report.vip_count = tickets[flags contains "vip"].count
             set: report.urgent_printer_count = tickets[title contains "printer" and tags contains "urgent"].count
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());

    let state: std::collections::BTreeMap<String, String> = [
        ("report.printer_count".to_string(), "0".to_string()),
        ("report.urgent_count".to_string(), "0".to_string()),
        ("report.vip_count".to_string(), "0".to_string()),
        ("report.urgent_printer_count".to_string(), "0".to_string()),
        ("tickets.T1.id".to_string(), "T1".to_string()),
        ("tickets.T1.title".to_string(), "printer jam".to_string()),
        (
            "tickets.T1.tags".to_string(),
            "[urgent,hardware]".to_string(),
        ),
        ("tickets.T1.flags".to_string(), "{\"vip\":true}".to_string()),
        ("tickets.T2.id".to_string(), "T2".to_string()),
        ("tickets.T2.title".to_string(), "printer toner".to_string()),
        ("tickets.T2.tags".to_string(), "[low,hardware]".to_string()),
        ("tickets.T2.flags".to_string(), "{\"sla\":true}".to_string()),
        ("tickets.T3.id".to_string(), "T3".to_string()),
        ("tickets.T3.title".to_string(), "network down".to_string()),
        (
            "tickets.T3.tags".to_string(),
            "[urgent,network]".to_string(),
        ),
        ("tickets.T3.flags".to_string(), "{\"sla\":true}".to_string()),
    ]
    .into();

    let simulated = simulate(&document, state.clone(), [].into());
    assert_eq!(simulated.status, "succeeded");
    assert_eq!(simulated.final_state["report.printer_count"], "2");
    assert_eq!(simulated.final_state["report.urgent_count"], "2");
    assert_eq!(simulated.final_state["report.vip_count"], "1");
    assert_eq!(simulated.final_state["report.urgent_printer_count"], "1");

    let bytecode = run_bytecode(&lower_document(&document), state, [].into());
    assert_eq!(bytecode.status, "succeeded");
    assert_eq!(bytecode.final_state["report.printer_count"], "2");
    assert_eq!(bytecode.final_state["report.urgent_count"], "2");
    assert_eq!(bytecode.final_state["report.vip_count"], "1");
    assert_eq!(bytecode.final_state["report.urgent_printer_count"], "1");
}

#[test]
fn collection_logical_filters_drive_requirements_and_runtime_flow() {
    let document = parse_rif_text(
        r#"
        app TicketFilteredMetrics

        things:
          thing Ticket
            field id: Text
            field status: State<Open, Closed>
            field priority: Int

          thing Report
            field selected_count: Int

        collections:
          collection tickets: Ticket

        intent CountSelectedTickets

        requires:
          tickets[status=Open and priority>=2].count is 1

        subject:
          report: Report

        steps:
          1. Count selected tickets
             set: report.selected_count = tickets[status=Open and priority>=2].count
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());

    let state: std::collections::BTreeMap<String, String> = [
        ("report.selected_count".to_string(), "0".to_string()),
        ("tickets.T1.id".to_string(), "T1".to_string()),
        ("tickets.T1.status".to_string(), "Open".to_string()),
        ("tickets.T1.priority".to_string(), "1".to_string()),
        ("tickets.T2.id".to_string(), "T2".to_string()),
        ("tickets.T2.status".to_string(), "Open".to_string()),
        ("tickets.T2.priority".to_string(), "3".to_string()),
        ("tickets.T3.id".to_string(), "T3".to_string()),
        ("tickets.T3.status".to_string(), "Closed".to_string()),
        ("tickets.T3.priority".to_string(), "5".to_string()),
    ]
    .into();

    let simulated = simulate(&document, state.clone(), [].into());
    assert_eq!(simulated.status, "succeeded");
    assert_eq!(simulated.final_state["report.selected_count"], "1");

    let bytecode = run_bytecode(&lower_document(&document), state, [].into());
    assert_eq!(bytecode.status, "succeeded");
    assert_eq!(bytecode.final_state["report.selected_count"], "1");
}

#[test]
fn collection_filters_accept_expression_thresholds() {
    let document = parse_rif_text(
        r#"
        app TicketExpressionFilters

        things:
          thing Ticket
            field id: Text
            field priority: Int

          thing Report
            field minimum_priority: Int
            field selected_count: Int

        collections:
          collection tickets: Ticket

        intent CountSelectedTickets

        requires:
          tickets[priority>=report.minimum_priority + 1].count is 1

        subject:
          report: Report

        steps:
          1. Count selected tickets
             set: report.selected_count = tickets[priority>=report.minimum_priority + 1].count
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());

    let state: std::collections::BTreeMap<String, String> = [
        ("report.minimum_priority".to_string(), "2".to_string()),
        ("report.selected_count".to_string(), "0".to_string()),
        ("tickets.T1.id".to_string(), "T1".to_string()),
        ("tickets.T1.priority".to_string(), "2".to_string()),
        ("tickets.T2.id".to_string(), "T2".to_string()),
        ("tickets.T2.priority".to_string(), "3".to_string()),
    ]
    .into();

    let simulated = simulate(&document, state.clone(), [].into());
    assert_eq!(simulated.status, "succeeded");
    assert_eq!(simulated.final_state["report.selected_count"], "1");

    let bytecode = run_bytecode(&lower_document(&document), state, [].into());
    assert_eq!(bytecode.status, "succeeded");
    assert_eq!(bytecode.final_state["report.selected_count"], "1");
}

#[test]
fn checker_reports_invalid_collection_selector_operand_types() {
    let document = parse_rif_text(
        r#"
        app BadCollectionSelectorTypes

        things:
          thing Ticket
            field id: Text
            field status: State<Open, Closed>

          thing Report
            field selected_count: Int

        collections:
          collection tickets: Ticket

        intent CountSelectedTickets

        requires:
          tickets[status>=0].count is 1

        subject:
          report: Report

        steps:
          1. Count selected tickets
             set: report.selected_count = tickets[status>=0].count
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_COLLECTION_SELECTOR_TYPE".to_string()));
}

#[test]
fn checker_reports_invalid_logical_collection_selector_operand_types() {
    let document = parse_rif_text(
        r#"
        app BadLogicalCollectionSelectorTypes

        things:
          thing Ticket
            field id: Text
            field status: State<Open, Closed>
            field priority: Int

          thing Report
            field selected_count: Int

        collections:
          collection tickets: Ticket

        intent CountSelectedTickets

        requires:
          tickets[priority>=2 and status>=0].count is 1

        subject:
          report: Report

        steps:
          1. Count selected tickets
             set: report.selected_count = tickets[priority>=2 and status>=0].count
        "#,
    )
    .unwrap();

    let selector_type_errors = check_document(&document)
        .into_iter()
        .filter(|diagnostic| diagnostic.code == "EIGL_COLLECTION_SELECTOR_TYPE")
        .count();
    assert_eq!(selector_type_errors, 2);
}

#[test]
fn bytecode_iterates_over_filtered_collection_records() {
    let document = parse_rif_file(fixture("ticket_iteration_app.rif.md")).unwrap();
    let bytecode = lower_document(&document);
    assert!(
        bytecode
            .instructions
            .iter()
            .any(|instruction| instruction.opcode == "ITERATE_BEGIN"
                && instruction.fields["query"] == "tickets[status=Closed]")
    );
    assert!(
        bytecode
            .instructions
            .iter()
            .any(|instruction| instruction.opcode == "ITERATE_END")
    );

    let result = run_bytecode(
        &bytecode,
        [
            ("report.closed_count".to_string(), "0".to_string()),
            ("tickets.T1.id".to_string(), "T1".to_string()),
            ("tickets.T1.status".to_string(), "Closed".to_string()),
            ("tickets.T1.title".to_string(), "Stale".to_string()),
            ("tickets.T2.id".to_string(), "T2".to_string()),
            ("tickets.T2.status".to_string(), "Open".to_string()),
            ("tickets.T2.title".to_string(), "Active".to_string()),
        ]
        .into(),
        [].into(),
    );

    assert_eq!(result.status, "succeeded");
    assert_eq!(result.final_state["report.closed_count"], "1");
    assert!(!result.final_state.contains_key("tickets.T1.status"));
    assert!(result.final_state.contains_key("tickets.T2.status"));
}

#[test]
fn parser_elaborates_prose_state_and_failure_sentences() {
    let document = parse_rsl_text(
        r#"
        intent Confirm Order

        subject:
          order: Order

        things:
          thing Order
            field status: State<Draft, Confirmed, PaidAwaitingFulfillmentRetry>

        A draft order can be confirmed when inventory is available and payment succeeds.

        If payment capture fails:
          release the inventory reservation.
        "#,
    )
    .unwrap();

    assert_eq!(
        document.intent.state_transitions[0],
        eigl::rif_model::StateTransition {
            field_path: "order.status".to_string(),
            from_state: "Draft".to_string(),
            to_state: "Confirmed".to_string(),
        }
    );
    assert!(
        document
            .intent
            .requires
            .iter()
            .any(|requirement| requirement.text == "inventory is available and payment succeeds")
    );
    assert_eq!(
        document.intent.failure_handlers[0].condition,
        "payment capture fails"
    );
    assert_eq!(
        document.intent.failure_handlers[0].actions,
        vec!["release the inventory reservation."]
    );
}

#[test]
fn parser_reports_unknown_phrase_in_compact_rsl() {
    let error = parse_rsl_text(
        r#"
        intent Unknown Phrase

        things:
          thing Order
            field status: State<Draft, Confirmed>

        subject:
          order: Order

        Draft -> Confirmed
        by reserve inventory
        "#,
    )
    .unwrap_err();

    assert!(error.contains("unknown phrase"));
    assert!(error.contains("reserve inventory"));
}

#[test]
fn parser_preserves_unknown_step_lines_and_secret_inputs() {
    let document = parse_rif_text(
        r#"
        intent UnknownLines

        inputs:
          new_password: Secret<Text>

        steps:
          1. Do work
             call: Worker.do(new_password)
             retry up to: 3 times

        guarantees:
          if this intent succeeds:
            new_password exists
        "#,
    )
    .unwrap();

    assert!(document.intent.inputs["new_password"].is_secret);
    assert_eq!(
        document.intent.steps[0].raw_lines,
        vec!["retry up to: 3 times"]
    );
}

#[test]
fn graph_builder_creates_core_graph_and_json() {
    let document = parse_rif_file(fixture("confirm_order.rif.md")).unwrap();
    let program = build_program(&document);
    let graph = &program.graph;

    assert!(graph.find_node("intent", "ConfirmOrder").is_some());
    assert!(graph.find_node("step", "Capture payment").is_some());
    assert!(graph.find_node("value", "payment").is_some());
    assert!(graph.find_node("field", "order.status").is_some());
    assert!(
        graph
            .find_node("permission", "Change PaymentLedger")
            .is_some()
    );
    assert!(graph.find_node("failure", "PaymentFailed").is_some());
    assert!(graph.has_edge("has_field", "order", "order.status"));
    assert!(graph.has_edge("has_state", "order.status", "Draft"));
    assert!(graph.has_edge("transitions_to", "Draft", "Confirmed"));
    assert!(graph.has_edge("calls", "Capture payment", "PaymentGateway.capture"));
    assert!(graph.has_edge("produces", "Capture payment", "payment"));
    assert!(graph.has_edge("may_fail", "Capture payment", "PaymentFailed"));
    assert!(graph.has_edge(
        "compensates",
        "Reserve inventory",
        "Inventory.release(reservation)"
    ));
    assert!(graph.validate_edge_references().is_empty());

    let json = program.to_json();
    assert!(json.contains("\"modules\""));
    assert!(json.contains("\"ConfirmOrder\""));
    assert_eq!(
        program.graph.node_ids(),
        build_program(&document).graph.node_ids(),
        "semantic node IDs must be stable"
    );
}

#[test]
fn checker_accepts_valid_examples_and_reports_conflicts() {
    let confirm_order = parse_rif_file(fixture("confirm_order.rif.md")).unwrap();
    assert_eq!(check_document(&confirm_order), Vec::new());

    let password_reset = parse_rif_file(fixture("password_reset.rif.md")).unwrap();
    assert_eq!(check_document(&password_reset), Vec::new());

    let bad_parallel = parse_rif_file(fixture("bad_parallel_update.rif.md")).unwrap();
    let diagnostics = check_document(&bad_parallel);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "EIGL_PERMISSION_CONFLICT");
    assert!(diagnostics[0].message.contains("order.status"));
    assert!(diagnostics[0].message.contains("Mark order paid"));
    assert!(diagnostics[0].message.contains("Mark order cancelled"));
}

#[test]
fn checker_rejects_duplicate_intent_names() {
    let document = parse_rif_text(
        r#"
        app DuplicateIntents

        intent RunJob

        intent RunJob
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_DUPLICATE_INTENT".to_string()));
}

#[test]
fn checker_reports_required_safety_diagnostics() {
    let cases = [
        (
            "EIGL_OUTPUT_TYPE_REQUIRED",
            r#"
            intent MissingOutputType
            subject:
              order: Order
            steps:
              1. Reserve inventory
                 call: Inventory.reserve(order.items)
                 output: reservation
            guarantees:
              if this intent succeeds:
                reservation exists
            "#,
        ),
        (
            "EIGL_UNHANDLED_FAILURE",
            r#"
            intent MissingFailureHandler
            subject:
              order: Order
            steps:
              1. Capture payment
                 call: PaymentGateway.capture(order.payment_method)
                 may fail with: PaymentFailed
            guarantees:
              if this intent succeeds:
                order exists
            "#,
        ),
        (
            "EIGL_SECRET_FLOW",
            r#"
            intent LeakyPassword
            inputs:
              new_password: Secret<Text>
            steps:
              1. Log password
                 call: EventLog.record(new_password)
                 external call: EventLog
            guarantees:
              if this intent succeeds:
                new_password exists
            "#,
        ),
        (
            "EIGL_COMPENSATION_VALUE_UNAVAILABLE",
            r#"
            intent BadCompensation
            subject:
              order: Order
            steps:
              1. Capture payment
                 call: PaymentGateway.capture(order.payment_method)
                 may fail with: PaymentFailed
                 compensation: PaymentGateway.refund(payment)
            failure behavior:
              if payment capture fails:
                stop with PaymentFailed
            guarantees:
              if this intent succeeds:
                order exists
            "#,
        ),
        (
            "EIGL_UNKNOWN_GUARANTEE_REFERENCE",
            r#"
            intent UnknownGuarantee
            subject:
              order: Order
            steps:
              1. Mark order paid
                 set: order.status = Paid
                 changes: order.status
            guarantees:
              if this intent succeeds:
                invoice exists
            "#,
        ),
    ];

    for (expected_code, source) in cases {
        let document = parse_rif_text(source).unwrap();
        let codes: Vec<_> = check_document(&document)
            .into_iter()
            .map(|diagnostic| diagnostic.code)
            .collect();
        assert!(
            codes.contains(&expected_code.to_string()),
            "expected {expected_code}, got {codes:?}"
        );
    }
}

#[test]
fn checker_reports_secret_field_flows() {
    let document = parse_rif_text(
        r#"
        app SecretFields

        things:
          thing User
            field password: Secret<Text>

        operations:
          operation Audit.log(password: Secret<Text>) -> Unit
          operation Hash.password(password: Secret<Text>) -> Text

        intent UpdatePassword

        subject:
          user: User

        steps:
          1. Log password
             call: Audit.log(user.password)
          2. Hash password
             call: Hash.password(user.password)
             output: password_hash: Text

        returns:
          leaked: user.password
        "#,
    )
    .unwrap();

    let diagnostics = check_document(&document);
    let secret_flow_count = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "EIGL_SECRET_FLOW")
        .count();
    assert_eq!(secret_flow_count, 2, "{diagnostics:#?}");
}

#[test]
fn checker_reports_secret_endpoint_response_flows() {
    let document = parse_rif_text(
        r#"
        app SecretEndpoint

        things:
          thing User
            field password: Secret<Text>

        endpoints:
          endpoint GET /me -> ShowUser
            respond:
              password: Secret<Text>
              password = user.password
            error:
              password: Secret<Text>
              password = user.password

        intent ShowUser

        subject:
          user: User
        "#,
    )
    .unwrap();

    let diagnostics = check_document(&document);
    let secret_flow_count = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "EIGL_SECRET_FLOW")
        .count();
    assert_eq!(secret_flow_count, 2, "{diagnostics:#?}");
}

#[test]
fn checker_reports_secret_object_endpoint_response_flows() {
    let document = parse_rif_text(
        r#"
        app SecretObjectEndpoint

        things:
          thing User
            field name: Text
            field password: Secret<Text>

        endpoints:
          endpoint GET /me -> ShowUser
            respond:
              user: User
              user = user

        intent ShowUser

        subject:
          user: User
        "#,
    )
    .unwrap();

    assert!(
        check_document(&document)
            .iter()
            .any(|diagnostic| diagnostic.code == "EIGL_SECRET_FLOW")
    );
}

#[test]
fn views_and_explanations_are_human_readable() {
    let document = parse_rif_file(fixture("confirm_order.rif.md")).unwrap();

    let flow = flow_view(&document);
    assert!(flow.contains("[Draft Order]"));
    assert!(flow.contains("[Reserve inventory]"));
    assert!(flow.contains("[Confirmed Order]"));
    assert!(flow.contains("↓"));

    let failures = failure_view(&document);
    assert!(failures.contains("payment capture fails"));
    assert!(failures.contains("release reservation"));
    assert!(failures.contains("Stop with PaymentFailed"));

    let permissions = permission_view(&document);
    assert!(permissions.contains("ConfirmOrder"));
    assert!(permissions.contains("reads:"));
    assert!(permissions.contains("order.payment_method"));
    assert!(!permissions.contains("Confirmed"));
    assert!(permissions.contains("changes:"));
    assert!(permissions.contains("ShipmentQueue"));

    let effects = effect_view(&document);
    assert_eq!(effects.matches("write order.status").count(), 1);

    let explanation = eigl::explanations::explain_intent(&document);
    assert!(explanation.contains("ConfirmOrder"));
    assert!(explanation.contains("checks order.status is Draft"));
    assert!(
        explanation
            .to_ascii_lowercase()
            .contains("reserves inventory")
    );
    assert!(explanation.contains("PaymentFailed"));
    assert!(explanation.contains("order.status is Confirmed"));

    let model = view_model(&document);
    assert_eq!(model.intent, "ConfirmOrder");
    assert_eq!(model.flow[0].kind, "state");
    assert_eq!(model.flow[1].label, "Reserve inventory");
}

#[test]
fn interpreter_simulates_success_and_failure_paths() {
    let document = parse_rif_file(fixture("confirm_order.rif.md")).unwrap();

    let success = simulate(
        &document,
        [
            ("order.status".to_string(), "Draft".to_string()),
            ("order.items.count".to_string(), "2".to_string()),
        ]
        .into(),
        [].into(),
    );
    assert_eq!(success.status, "succeeded");
    assert_eq!(success.final_state["order.status"], "Confirmed");
    assert!(success.outputs.contains_key("payment"));
    assert!(success.outputs.contains_key("shipment"));
    assert_eq!(success.failure, None);

    let payment_failure = simulate(
        &document,
        [
            ("order.status".to_string(), "Draft".to_string()),
            ("order.items.count".to_string(), "2".to_string()),
        ]
        .into(),
        [("Capture payment".to_string(), "PaymentFailed".to_string())].into(),
    );
    assert_eq!(payment_failure.status, "failed");
    assert_eq!(payment_failure.failure.as_deref(), Some("PaymentFailed"));
    assert!(
        payment_failure
            .trace
            .contains(&"release reservation".to_string())
    );
    assert!(!payment_failure.outputs.contains_key("shipment"));

    let shipping_failure = simulate(
        &document,
        [
            ("order.status".to_string(), "Draft".to_string()),
            ("order.items.count".to_string(), "2".to_string()),
        ]
        .into(),
        [(
            "Create shipment request".to_string(),
            "ShipmentFailed".to_string(),
        )]
        .into(),
    );
    assert_eq!(shipping_failure.status, "failed");
    assert_eq!(
        shipping_failure.failure.as_deref(),
        Some("FulfillmentRetryNeeded")
    );
    assert_eq!(
        shipping_failure.final_state["order.status"],
        "PaidAwaitingFulfillmentRetry"
    );
}

#[test]
fn interpreter_enforces_requirements_without_defaulting_state() {
    let document = parse_rif_file(fixture("invoice_app.rif.md")).unwrap();

    let missing_tax = simulate(
        &document,
        [
            ("invoice.status".to_string(), "Draft".to_string()),
            ("invoice.subtotal".to_string(), "10".to_string()),
        ]
        .into(),
        [].into(),
    );

    assert_eq!(missing_tax.status, "failed");
    assert_eq!(missing_tax.failure.as_deref(), Some("RequirementFailed"));
    assert!(
        missing_tax
            .trace
            .contains(&"CHECK FAILED invoice.tax > 0".to_string())
    );
    assert!(
        !missing_tax
            .trace
            .contains(&"Calculate invoice total".to_string())
    );
}

#[test]
fn interpreter_evaluates_compound_requirements() {
    let document = parse_rif_text(
        r#"
        app CompoundSimulationRequirements

        things:
          thing Invoice
            field status: State<Draft, Finalized>
            field subtotal: Int
            field total: Int

        intent FinalizeInvoice

        subject:
          invoice: Invoice

        requires:
          invoice.status is Draft and invoice.subtotal > 0

        steps:
          1. Set total
             set: invoice.total = 1
             changes: invoice.total

        guarantees:
          if this intent succeeds:
            invoice.total > 0
        "#,
    )
    .unwrap();

    let success = simulate(
        &document,
        [
            ("invoice.status".to_string(), "Draft".to_string()),
            ("invoice.subtotal".to_string(), "10".to_string()),
        ]
        .into(),
        [].into(),
    );
    assert_eq!(success.status, "succeeded");

    let failed = simulate(
        &document,
        [
            ("invoice.status".to_string(), "Draft".to_string()),
            ("invoice.subtotal".to_string(), "0".to_string()),
        ]
        .into(),
        [].into(),
    );
    assert_eq!(failed.status, "failed");
    assert_eq!(failed.failure.as_deref(), Some("RequirementFailed"));
}

#[test]
fn interpreter_evaluates_negated_and_grouped_requirements() {
    let document = parse_rif_text(
        r#"
        app GroupedSimulationRequirements

        things:
          thing Invoice
            field status: State<Draft, Finalized>
            field subtotal: Int
            field tax: Int
            field total: Int

        intent FinalizeInvoice

        subject:
          invoice: Invoice

        requires:
          not (invoice.status is Finalized) and (invoice.subtotal > 0 or invoice.tax > 0)

        steps:
          1. Set total
             set: invoice.total = 1
             changes: invoice.total

        guarantees:
          if this intent succeeds:
            invoice.total > 0
        "#,
    )
    .unwrap();

    let success = simulate(
        &document,
        [
            ("invoice.status".to_string(), "Draft".to_string()),
            ("invoice.subtotal".to_string(), "0".to_string()),
            ("invoice.tax".to_string(), "1".to_string()),
        ]
        .into(),
        [].into(),
    );
    assert_eq!(success.status, "succeeded");

    let failed = simulate(
        &document,
        [
            ("invoice.status".to_string(), "Finalized".to_string()),
            ("invoice.subtotal".to_string(), "10".to_string()),
            ("invoice.tax".to_string(), "0".to_string()),
        ]
        .into(),
        [].into(),
    );
    assert_eq!(failed.status, "failed");
    assert_eq!(failed.failure.as_deref(), Some("RequirementFailed"));
}

#[test]
fn eig_ir_lowers_rif_to_executable_bytecode() {
    let document = parse_rif_file(fixture("confirm_order.rif.md")).unwrap();
    let bytecode = lower_document(&document);

    assert_eq!(bytecode.intent, "ConfirmOrder");
    assert_eq!(bytecode.instructions[0].opcode, "CHECK_REQUIRES");
    assert!(
        bytecode
            .instructions
            .iter()
            .any(|instruction| instruction.opcode == "CALL_EFFECT"
                && instruction.fields["step"] == "Capture payment"
                && instruction.fields["target"] == "PaymentGateway.capture")
    );
    assert!(
        bytecode
            .instructions
            .iter()
            .any(|instruction| instruction.opcode == "STORE_OUTPUT"
                && instruction.fields["name"] == "payment"
                && instruction.fields["type"] == "PaymentCapture"
                && instruction.fields["step_number"] == "2")
    );
    assert!(
        bytecode
            .instructions
            .iter()
            .any(|instruction| instruction.opcode == "REGISTER_COMPENSATION"
                && instruction.fields["expression"] == "Inventory.release(reservation)")
    );
    assert!(
        bytecode
            .instructions
            .iter()
            .any(|instruction| instruction.opcode == "HANDLE_FAILURE"
                && instruction.fields["condition"] == "payment capture fails")
    );
    assert_eq!(bytecode.instructions.last().unwrap().opcode, "RETURN");

    let json = bytecode.to_json();
    assert!(json.contains("\"opcode\":\"CALL_EFFECT\""));
    assert!(json.contains("\"intent\":\"ConfirmOrder\""));
}

#[test]
fn bytecode_vm_runs_success_and_failure_paths() {
    let document = parse_rif_file(fixture("confirm_order.rif.md")).unwrap();
    let bytecode = lower_document(&document);

    let success = run_bytecode(
        &bytecode,
        [
            ("order.status".to_string(), "Draft".to_string()),
            ("order.items.count".to_string(), "2".to_string()),
        ]
        .into(),
        [].into(),
    );
    assert_eq!(success.status, "succeeded");
    assert_eq!(success.final_state["order.status"], "Confirmed");
    assert_eq!(success.outputs["payment"], "payment#2");
    assert_eq!(success.outputs["shipment"], "shipment#3");
    assert!(
        success
            .trace
            .iter()
            .any(|entry| entry == "CALL PaymentGateway.capture")
    );

    let payment_failure = run_bytecode(
        &bytecode,
        [
            ("order.status".to_string(), "Draft".to_string()),
            ("order.items.count".to_string(), "2".to_string()),
        ]
        .into(),
        [("Capture payment".to_string(), "PaymentFailed".to_string())].into(),
    );
    assert_eq!(payment_failure.status, "failed");
    assert_eq!(payment_failure.failure.as_deref(), Some("PaymentFailed"));
    assert!(
        payment_failure
            .trace
            .contains(&"release reservation".to_string())
    );
    assert!(!payment_failure.outputs.contains_key("shipment"));

    let shipping_failure = run_bytecode(
        &bytecode,
        [
            ("order.status".to_string(), "Draft".to_string()),
            ("order.items.count".to_string(), "2".to_string()),
        ]
        .into(),
        [(
            "Create shipment request".to_string(),
            "ShipmentFailed".to_string(),
        )]
        .into(),
    );
    assert_eq!(shipping_failure.status, "failed");
    assert_eq!(
        shipping_failure.failure.as_deref(),
        Some("FulfillmentRetryNeeded")
    );
    assert_eq!(
        shipping_failure.final_state["order.status"],
        "PaidAwaitingFulfillmentRetry"
    );
}

#[test]
fn application_workflow_runs_through_bytecode_backend() {
    let document = parse_rif_file(fixture("issue_tracker.rif.md")).unwrap();
    assert_eq!(check_document(&document), Vec::new());

    let bytecode = lower_document(&document);
    assert!(
        bytecode
            .instructions
            .iter()
            .any(|instruction| instruction.opcode == "CALL_EFFECT"
                && instruction.fields["target"] == "IssueComments.add")
    );

    let success = run_bytecode(
        &bytecode,
        [
            ("issue.status".to_string(), "Open".to_string()),
            ("issue.assignee".to_string(), "user-1".to_string()),
            ("actor.id".to_string(), "user-1".to_string()),
        ]
        .into(),
        [].into(),
    );
    assert_eq!(success.status, "succeeded");
    assert_eq!(success.final_state["issue.status"], "Resolved");
    assert_eq!(success.outputs["comment"], "comment#2");

    let unauthorized = run_bytecode(
        &bytecode,
        [("issue.status".to_string(), "Open".to_string())].into(),
        [("Check assignee".to_string(), "NotAssignee".to_string())].into(),
    );
    assert_eq!(unauthorized.status, "failed");
    assert_eq!(unauthorized.failure.as_deref(), Some("NotAssignee"));
    assert!(!unauthorized.outputs.contains_key("comment"));
}

#[test]
fn application_rif_declares_domain_model_operations_and_multiple_intents() {
    let document = parse_rif_file(fixture("issue_tracker_app.rif.md")).unwrap();

    assert_eq!(document.application.name.as_deref(), Some("IssueTracker"));
    assert_eq!(document.intents.len(), 2);
    assert_eq!(document.intent.name, "ResolveIssue");
    assert_eq!(document.intents[1].name, "ReopenIssue");
    assert_eq!(
        document.application.things["Issue"].fields["status"].type_name,
        "State<Open, Resolved, Closed>"
    );
    assert!(
        document
            .application
            .operations
            .contains_key("IssuePolicy.require_assignee")
    );
    assert_eq!(
        document.application.operations["IssueComments.add"].outputs[0].type_name,
        "Comment"
    );

    assert_eq!(check_document(&document), Vec::new());
    let program = build_program(&document);
    assert_eq!(program.modules[0].name, "IssueTracker");
    assert_eq!(
        program.modules[0].intents,
        vec!["ResolveIssue".to_string(), "ReopenIssue".to_string()]
    );
    assert!(program.graph.find_node("thing", "Issue").is_some());
    assert!(program.graph.has_edge("has_field", "Issue", "Issue.status"));
    assert!(
        program
            .graph
            .has_edge("calls", "Check assignee", "IssuePolicy.require_assignee")
    );
    assert!(
        program
            .graph
            .has_edge("may_fail", "Check assignee", "NotAssignee")
    );
    assert!(
        program
            .permissions
            .iter()
            .any(|permission| permission.kind == "Read" && permission.target == "issue.assignee")
    );
    assert!(
        program
            .permissions
            .iter()
            .any(|permission| permission.kind == "Change" && permission.target == "IssueComments")
    );

    let bytecode = lower_document(&document);
    assert_eq!(bytecode.intent, "ResolveIssue");
    assert!(
        bytecode
            .instructions
            .iter()
            .any(|instruction| instruction.opcode == "CALL_EFFECT"
                && instruction.fields["step"] == "Notify watchers"
                && instruction.fields["may_fail"] == "NotificationFailed")
    );
}

#[test]
fn checker_reports_unknown_declared_type_and_unknown_operation() {
    let document = parse_rif_text(
        r#"
        app BrokenApp

        things:
          thing Task
            field owner: MissingUser

        intent CloseTask

        subject:
          task: Task

        steps:
          1. Close task
             call: MissingOps.close(task)
             output: closed: MissingResult

        guarantees:
          if this intent succeeds:
            task exists
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_UNKNOWN_TYPE".to_string()));
    assert!(codes.contains(&"EIGL_UNKNOWN_OPERATION".to_string()));
}

#[test]
fn checker_validates_operation_call_argument_signatures() {
    let document = parse_rif_text(
        r#"
        app BrokenCalls

        things:
          thing Issue
            field id: Id<Issue>

          thing User
            field id: Id<User>

        operations:
          operation IssueComments.add(issue_id: Id<Issue>, body: Text) -> Unit

        intent ResolveIssue

        subject:
          issue: Issue

        inputs:
          actor: User

        steps:
          1. Missing body
             call: IssueComments.add(issue.id)

          2. Wrong body type
             call: IssueComments.add(issue.id, actor)

          3. Unknown body value
             call: IssueComments.add(issue.id, missing_note)

        guarantees:
          if this intent succeeds:
            issue exists
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_OPERATION_ARGUMENT_COUNT".to_string()));
    assert!(codes.contains(&"EIGL_OPERATION_ARGUMENT_TYPE".to_string()));
    assert!(codes.contains(&"EIGL_UNKNOWN_CALL_ARGUMENT_REFERENCE".to_string()));
}

#[test]
fn checker_reports_invalid_operation_permission_targets() {
    let document = parse_rif_text(
        r#"
        app BrokenOperationPermissions

        things:
          thing Invoice
            field subtotal: Int
            field total: Int

        operations:
          operation Billing.audit(invoice: Invoice) -> Unit
            reads: invoice.missing
            changes: invoice.unknown

        intent AuditInvoice

        subject:
          invoice: Invoice

        steps:
          1. Audit invoice
             call: Billing.audit(invoice)
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_UNKNOWN_OPERATION_READ_TARGET".to_string()));
    assert!(codes.contains(&"EIGL_UNKNOWN_OPERATION_CHANGE_TARGET".to_string()));
}

#[test]
fn operation_calls_accept_expression_arguments() {
    let document = parse_rif_text(
        r#"
        app OperationExpressionArgs

        things:
          thing Invoice
            field subtotal: Int
            field tax: Int

        operations:
          operation Billing.record(total: Int) -> Unit

        intent RecordInvoice

        subject:
          invoice: Invoice

        steps:
          1. Record computed total
             call: Billing.record(invoice.subtotal + invoice.tax)
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());

    let bytecode = lower_document(&document);
    assert!(bytecode.instructions.iter().any(|instruction| {
        instruction.opcode == "CALL_EFFECT"
            && instruction.fields["target"] == "Billing.record"
            && instruction.fields["args"] == "invoice.subtotal + invoice.tax"
    }));
}

#[test]
fn checker_validates_operation_call_output_contracts() {
    let document = parse_rif_text(
        r#"
        app BrokenOutputs

        things:
          thing Issue
            field id: Id<Issue>

          thing Comment
            field id: Id<Comment>

          thing User
            field id: Id<User>

        operations:
          operation IssueComments.add(issue_id: Id<Issue>) -> Comment

          operation Notification.send(issue_id: Id<Issue>) -> Unit

        intent ResolveIssue

        subject:
          issue: Issue

        steps:
          1. Add comment with wrong output
             call: IssueComments.add(issue.id)
             output: user: User

          2. Store unit output
             call: Notification.send(issue.id)
             output: sent: Comment

        guarantees:
          if this intent succeeds:
            issue exists
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_OPERATION_OUTPUT_TYPE".to_string()));
    assert!(codes.contains(&"EIGL_OPERATION_OUTPUT_COUNT".to_string()));
}

#[test]
fn operation_named_outputs_round_trip_check_and_execute() {
    let document = parse_rif_text(
        r#"
        app PaymentAuthorization

        things:
          thing Invoice
            field total: Money

        operations:
          operation Payment.authorize(amount: Money)
            output: authorization_id: Text
            output: risk_score: Decimal

        intent AuthorizePayment

        subject:
          invoice: Invoice

        steps:
          1. Authorize payment
             call: Payment.authorize(invoice.total)
             output: authorization_id: Text
             output: risk_score: Decimal

        returns:
          authorization: authorization_id
          risk: risk_score

        guarantees:
          if this intent succeeds:
            authorization_id exists
        "#,
    )
    .unwrap();

    let operation = &document.application.operations["Payment.authorize"];
    assert_eq!(
        operation
            .outputs
            .iter()
            .map(|output| (output.name.as_str(), output.type_name.as_str()))
            .collect::<Vec<_>>(),
        vec![("authorization_id", "Text"), ("risk_score", "Decimal")]
    );
    assert_eq!(check_document(&document), Vec::new());

    let rendered = render_rif_document(&document);
    assert!(rendered.contains("operation Payment.authorize(amount: Money)"));
    assert!(rendered.contains("output: authorization_id: Text"));
    assert!(rendered.contains("output: risk_score: Decimal"));
    let reparsed = parse_rif_text(&rendered).unwrap();
    assert_eq!(reparsed, document);

    let result = run_bytecode(
        &lower_document(&document),
        [("invoice.total".to_string(), "USD:20.00".to_string())].into(),
        [].into(),
    );
    assert_eq!(result.status, "succeeded");
    assert_eq!(result.outputs["authorization"], "authorization_id#1");
    assert_eq!(result.outputs["risk"], "risk_score#1");
}

#[test]
fn operation_object_outputs_can_assign_whole_typed_objects() {
    let document = parse_rif_text(
        r#"
        app CatalogTicketLookup

        things:
          thing Ticket
            field id: Text
            field title: Text

        operations:
          operation Catalog.fetch(id: Text) -> Ticket

        intent FillTicket

        subject:
          ticket: Ticket

        steps:
          1. Fetch ticket
             call: Catalog.fetch(ticket.id)
             output: fetched_ticket: Ticket

          2. Store returned ticket
             set: ticket = fetched_ticket
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());
    let state: std::collections::BTreeMap<String, String> = [
        ("ticket.id".to_string(), "T-1".to_string()),
        ("ticket.title".to_string(), "Old".to_string()),
    ]
    .into();
    let operation_outputs: std::collections::BTreeMap<String, String> = [(
        "fetched_ticket".to_string(),
        r#"{"id":"T-1","title":"Printer"}"#.to_string(),
    )]
    .into();

    let bytecode = run_bytecode_with_operation_outputs(
        &lower_document(&document),
        state.clone(),
        operation_outputs.clone(),
        Default::default(),
    );
    assert_eq!(bytecode.status, "succeeded");
    assert_eq!(bytecode.final_state["ticket.id"], "T-1");
    assert_eq!(bytecode.final_state["ticket.title"], "Printer");

    let simulation =
        simulate_with_operation_outputs(&document, state, operation_outputs, Default::default());
    assert_eq!(simulation.status, "succeeded");
    assert_eq!(simulation.final_state["ticket.id"], "T-1");
    assert_eq!(simulation.final_state["ticket.title"], "Printer");
}

#[test]
fn checker_validates_named_operation_output_contracts() {
    let document = parse_rif_text(
        r#"
        app BrokenPaymentAuthorization

        things:
          thing Invoice
            field total: Money

        operations:
          operation Payment.authorize(amount: Money)
            output: authorization_id: Text
            output: risk_score: Decimal

        intent AuthorizePayment

        subject:
          invoice: Invoice

        steps:
          1. Store wrong output type
             call: Payment.authorize(invoice.total)
             output: authorization_id: Decimal
             output: risk_score: Decimal

          2. Store unknown output
             call: Payment.authorize(invoice.total)
             output: authorization_id: Text
             output: approved: Bool

        guarantees:
          if this intent succeeds:
            invoice exists
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_OPERATION_OUTPUT_TYPE".to_string()));
    assert!(codes.contains(&"EIGL_OPERATION_OUTPUT_NAME".to_string()));
}

#[test]
fn checker_rejects_duplicate_operation_output_names() {
    let document = parse_rif_text(
        r#"
        app DuplicateOperationOutputs

        operations:
          operation Catalog.lookup(id: Text)
            output: title: Text
            output: title: Text

        intent LookupTicket
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_DUPLICATE_OPERATION_OUTPUT".to_string()));
}

#[test]
fn checker_requires_storing_named_operation_outputs() {
    let document = parse_rif_text(
        r#"
        app MissingPaymentAuthorizationOutputs

        things:
          thing Invoice
            field total: Money

        operations:
          operation Payment.authorize(amount: Money)
            output: authorization_id: Text
            output: risk_score: Decimal

        intent AuthorizePayment

        subject:
          invoice: Invoice

        steps:
          1. Authorize payment without storing returned values
             call: Payment.authorize(invoice.total)

        guarantees:
          if this intent succeeds:
            invoice exists
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_OPERATION_OUTPUT_COUNT".to_string()));
}

#[test]
fn checker_validates_set_assignment_types() {
    let document = parse_rif_text(
        r#"
        app BrokenAssignments

        things:
          thing Invoice
            field total: Int
            field status: State<Draft, Finalized>
            field note: Text

        intent FinalizeInvoice

        subject:
          invoice: Invoice

        steps:
          1. Set invalid state
             set: invoice.status = Paid
             changes: invoice.status

          2. Set invalid int
             set: invoice.total = "forty"
             changes: invoice.total

          3. Set from unknown value
             set: invoice.note = missing_note
             changes: invoice.note

        guarantees:
          if this intent succeeds:
            invoice exists
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_ASSIGNMENT_STATE_VALUE".to_string()));
    assert!(codes.contains(&"EIGL_ASSIGNMENT_TYPE".to_string()));
    assert!(codes.contains(&"EIGL_UNKNOWN_ASSIGNMENT_REFERENCE".to_string()));
}

#[test]
fn set_steps_copy_typed_object_fields() {
    let document = parse_rif_text(
        r#"
        app ObjectCopy

        things:
          thing Ticket
            field id: Text
            field title: Text
            field estimate: Int

        intent CopyTicket

        subject:
          source: Ticket
          draft: Ticket

        steps:
          1. Copy source into draft
             set: draft = source
             changes: draft
        "#,
    )
    .unwrap();
    assert_eq!(check_document(&document), Vec::new());

    let state: std::collections::BTreeMap<String, String> = [
        ("source.id".to_string(), "T-1".to_string()),
        ("source.title".to_string(), "Printer".to_string()),
        ("source.estimate".to_string(), "3".to_string()),
        ("draft.id".to_string(), "old".to_string()),
        ("draft.title".to_string(), "Old title".to_string()),
        ("draft.estimate".to_string(), "1".to_string()),
    ]
    .into();

    let bytecode = run_bytecode(&lower_document(&document), state.clone(), [].into());
    assert_eq!(bytecode.status, "succeeded");
    assert_eq!(bytecode.final_state["draft.id"], "T-1");
    assert_eq!(bytecode.final_state["draft.title"], "Printer");
    assert_eq!(bytecode.final_state["draft.estimate"], "3");

    let simulated = simulate(&document, state, [].into());
    assert_eq!(simulated.status, "succeeded");
    assert_eq!(simulated.final_state["draft.id"], "T-1");
    assert_eq!(simulated.final_state["draft.title"], "Printer");
    assert_eq!(simulated.final_state["draft.estimate"], "3");
}

#[test]
fn set_assignments_accept_arithmetic_expressions() {
    let document = parse_rif_text(
        r#"
        app AssignmentExpressions

        things:
          thing Invoice
            field subtotal: Int
            field tax: Int
            field total: Int

        intent FinalizeInvoice

        subject:
          invoice: Invoice

        steps:
          1. Set total from parts
             set: invoice.total = invoice.subtotal + invoice.tax
             changes: invoice.total
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());
    let program = build_program(&document);
    for target in ["invoice.subtotal", "invoice.tax"] {
        assert!(
            program
                .permissions
                .iter()
                .any(|permission| permission.kind == "Read" && permission.target == target),
            "missing read permission for {target}"
        );
    }

    let result = run_bytecode(
        &lower_document(&document),
        [
            ("invoice.subtotal".to_string(), "19".to_string()),
            ("invoice.tax".to_string(), "4".to_string()),
        ]
        .into(),
        [].into(),
    );

    assert_eq!(result.status, "succeeded");
    assert_eq!(result.final_state["invoice.total"], "23");
}

#[test]
fn checker_accepts_bool_literals_in_assignments_and_operation_calls() {
    let document = parse_rif_text(
        r#"
        app FeatureFlags

        things:
          thing Feature
            field enabled: Bool

        operations:
          operation Audit.record(enabled: Bool) -> Unit

        intent EnableFeature

        subject:
          feature: Feature

        steps:
          1. Enable feature
             set: feature.enabled = true
             changes: feature.enabled

          2. Audit previous state
             call: Audit.record(false)

        guarantees:
          if this intent succeeds:
            feature exists
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());
}

#[test]
fn checker_accepts_decimal_literals_and_int_to_decimal_widening() {
    let document = parse_rif_text(
        r#"
        app Pricing

        things:
          thing Product
            field price: Decimal
            field discount_rate: Decimal
            field quantity: Int

        operations:
          operation Pricing.apply(rate: Decimal) -> Unit
          operation Pricing.count(quantity: Decimal) -> Unit

        intent PriceProduct

        subject:
          product: Product

        steps:
          1. Set price
             set: product.price = 12.50
             changes: product.price

          2. Set discount rate
             set: product.discount_rate = 1
             changes: product.discount_rate

          3. Apply rate
             call: Pricing.apply(0.25)

          4. Count with widened integer
             call: Pricing.count(product.quantity)

        guarantees:
          if this intent succeeds:
            product exists
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());
}

#[test]
fn checker_accepts_money_literals_in_assignments_and_operation_calls() {
    let document = parse_rif_text(
        r#"
        app Billing

        things:
          thing Invoice
            field total: Money

        operations:
          operation Payment.capture(amount: Money) -> Unit

        intent CaptureInvoice

        subject:
          invoice: Invoice

        steps:
          1. Set total
             set: invoice.total = USD:12.50
             changes: invoice.total

          2. Capture payment
             call: Payment.capture(USD:12.50)

        guarantees:
          if this intent succeeds:
            invoice exists
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());
}

#[test]
fn checker_accepts_time_and_duration_literals_in_assignments_and_operation_calls() {
    let document = parse_rif_text(
        r#"
        app Scheduling

        things:
          thing Job
            field starts_at: Time
            field timeout: Duration

        operations:
          operation Scheduler.schedule(at: Time, timeout: Duration) -> Unit

        intent ScheduleJob

        subject:
          job: Job

        steps:
          1. Set start time
             set: job.starts_at = 2026-05-20T09:30:00Z
             changes: job.starts_at

          2. Set timeout
             set: job.timeout = PT30M
             changes: job.timeout

          3. Schedule job
             call: Scheduler.schedule(2026-05-20T09:30:00Z, PT30M)

        guarantees:
          if this intent succeeds:
            job exists
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());
}

#[test]
fn checker_accepts_list_literals_and_preserves_call_argument_commas() {
    let document = parse_rif_text(
        r#"
        app BulkImports

        things:
          thing Batch
            field counts: List<Int>

        operations:
          operation Bulk.add(counts: List<Int>) -> Unit

        intent ImportBatch

        subject:
          batch: Batch

        steps:
          1. Set counts
             set: batch.counts = [1,2,3]
             changes: batch.counts

          2. Add counts
             call: Bulk.add([1,2,3])

        guarantees:
          if this intent succeeds:
            batch exists
        "#,
    )
    .unwrap();

    assert_eq!(
        document.intent.steps[1].call.as_ref().unwrap().args,
        vec!["[1,2,3]"]
    );
    assert_eq!(check_document(&document), Vec::new());
}

#[test]
fn aggregate_literals_resolve_runtime_expressions() {
    let document = parse_rif_text(
        r#"
        app ProfileSummary

        things:
          thing Profile
            field first_name: Text
            field last_name: Text
            field names: List<Text>
            field counts: Map<Text, Int>
            field open_count: Int
            field closed_count: Int

        intent SummarizeProfile

        subject:
          profile: Profile

        steps:
          1. Build aggregate values
             set: profile.names = [profile.first_name,profile.last_name]
             set: profile.counts = {"open":profile.open_count,"closed":profile.closed_count}

        returns:
          first: profile.names[0]
          open: profile.counts["open"]
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());
    let state: std::collections::BTreeMap<String, String> = [
        ("profile.first_name".to_string(), "Ada".to_string()),
        ("profile.last_name".to_string(), "Lovelace".to_string()),
        ("profile.names".to_string(), "[]".to_string()),
        ("profile.counts".to_string(), "{}".to_string()),
        ("profile.open_count".to_string(), "2".to_string()),
        ("profile.closed_count".to_string(), "1".to_string()),
    ]
    .into();

    let result = run_bytecode(
        &lower_document(&document),
        state.clone(),
        Default::default(),
    );
    assert_eq!(result.status, "succeeded");
    assert_eq!(result.final_state["profile.names"], "[Ada,Lovelace]");
    assert_eq!(
        result.final_state["profile.counts"],
        "{\"open\":2,\"closed\":1}"
    );
    assert_eq!(result.outputs["first"], "Ada");
    assert_eq!(result.outputs["open"], "2");

    let simulation = simulate(&document, state, Default::default());
    assert_eq!(simulation.status, "succeeded");
    assert_eq!(simulation.final_state["profile.names"], "[Ada,Lovelace]");
    assert_eq!(
        simulation.final_state["profile.counts"],
        "{\"open\":2,\"closed\":1}"
    );
    assert_eq!(simulation.outputs["first"], "Ada");
    assert_eq!(simulation.outputs["open"], "2");
}

#[test]
fn list_appends_accept_typed_object_values() {
    let document = parse_rif_text(
        r#"
        app ThreadComments

        things:
          thing Comment
            field id: Text
            field text: Text

          thing Thread
            field comments: List<Comment>

        intent AddComment

        subject:
          thread: Thread
          comment: Comment

        steps:
          1. Add comment
             append: thread.comments += comment
             changes: thread.comments
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());

    let state: std::collections::BTreeMap<String, String> = [
        ("thread.comments".to_string(), "[]".to_string()),
        ("comment.id".to_string(), "C1".to_string()),
        ("comment.text".to_string(), "First".to_string()),
    ]
    .into();

    let result = run_bytecode(
        &lower_document(&document),
        state.clone(),
        Default::default(),
    );
    assert_eq!(
        result.final_state["thread.comments"],
        r#"[{"id":"C1","text":"First"}]"#
    );

    let simulation = simulate(&document, state, Default::default());
    assert_eq!(
        simulation.final_state["thread.comments"],
        r#"[{"id":"C1","text":"First"}]"#
    );
}

#[test]
fn list_literals_accept_typed_object_values() {
    let document = parse_rif_text(
        r#"
        app ThreadCommentLiterals

        things:
          thing Comment
            field id: Text
            field text: Text

          thing Thread
            field comments: List<Comment>

        intent SeedThread

        subject:
          thread: Thread
          comment: Comment

        steps:
          1. Seed comments
             set: thread.comments = [comment]
             changes: thread.comments
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());

    let state: std::collections::BTreeMap<String, String> = [
        ("thread.comments".to_string(), "[]".to_string()),
        ("comment.id".to_string(), "C1".to_string()),
        ("comment.text".to_string(), "First".to_string()),
    ]
    .into();

    let result = run_bytecode(
        &lower_document(&document),
        state.clone(),
        Default::default(),
    );
    assert_eq!(
        result.final_state["thread.comments"],
        r#"[{"id":"C1","text":"First"}]"#
    );

    let simulation = simulate(&document, state, Default::default());
    assert_eq!(
        simulation.final_state["thread.comments"],
        r#"[{"id":"C1","text":"First"}]"#
    );
}

#[test]
fn list_object_element_fields_are_typed_and_resolved() {
    let document = parse_rif_text(
        r#"
        app ThreadCommentFieldAccess

        things:
          thing Comment
            field id: Text
            field text: Text

          thing Thread
            field comments: List<Comment>
            field first_comment_text: Text

        intent SeedThreadSummary

        subject:
          thread: Thread

        steps:
          1. Copy first comment text
             when: thread.comments[0].text contains "First"
             set: thread.first_comment_text = thread.comments[0].text
             changes: thread.first_comment_text

          2. Update first comment text
             set: thread.comments[0].text = "Reviewed"
             changes: thread.comments

        returns:
          first_text: thread.comments[0].text
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());

    let state: std::collections::BTreeMap<String, String> = [(
        "thread.comments".to_string(),
        r#"[{"id":"C1","text":"First"}]"#.to_string(),
    )]
    .into();

    let result = run_bytecode(
        &lower_document(&document),
        state.clone(),
        Default::default(),
    );
    assert_eq!(result.final_state["thread.first_comment_text"], "First");
    assert_eq!(
        result.final_state["thread.comments"],
        r#"[{"id":"C1","text":"Reviewed"}]"#
    );
    assert_eq!(result.outputs["first_text"], "Reviewed");
    assert!(!result.final_state.contains_key("thread.comments[0].text"));

    let simulation = simulate(&document, state, Default::default());
    assert_eq!(simulation.final_state["thread.first_comment_text"], "First");
    assert_eq!(
        simulation.final_state["thread.comments"],
        r#"[{"id":"C1","text":"Reviewed"}]"#
    );
    assert_eq!(simulation.outputs["first_text"], "Reviewed");
    assert!(
        !simulation
            .final_state
            .contains_key("thread.comments[0].text")
    );
}

#[test]
fn list_append_steps_update_aggregate_state() {
    let document = parse_rif_text(
        r#"
        app ActivityLog

        things:
          thing Profile
            field events: List<Text>
            field next_event: Text

        intent RecordActivity

        subject:
          profile: Profile

        steps:
          1. Append event
             append: profile.events += profile.next_event
             changes: profile.events
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());
    let rendered = render_rif_document(&document);
    assert!(rendered.contains("append: profile.events += profile.next_event"));

    let state: std::collections::BTreeMap<String, String> = [
        ("profile.events".to_string(), "[created]".to_string()),
        ("profile.next_event".to_string(), "login".to_string()),
    ]
    .into();

    let result = run_bytecode(
        &lower_document(&document),
        state.clone(),
        Default::default(),
    );
    assert_eq!(result.status, "succeeded");
    assert_eq!(result.final_state["profile.events"], "[created,login]");

    let simulation = simulate(&document, state, Default::default());
    assert_eq!(simulation.status, "succeeded");
    assert_eq!(simulation.final_state["profile.events"], "[created,login]");
}

#[test]
fn list_appends_infer_read_permissions_from_expressions() {
    let document = parse_rif_text(
        r#"
        app ActivityLogExpressions

        things:
          thing Profile
            field events: List<Text>
            field prefix: Text
            field suffix: Text

        intent RecordActivity

        subject:
          profile: Profile

        steps:
          1. Append event
             append: profile.events += profile.prefix + profile.suffix
             changes: profile.events
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());
    let program = build_program(&document);
    for target in ["profile.prefix", "profile.suffix"] {
        assert!(
            program
                .permissions
                .iter()
                .any(|permission| permission.kind == "Read" && permission.target == target),
            "missing read permission for {target}"
        );
    }
}

#[test]
fn list_index_expressions_drive_runtime_flow() {
    let document = parse_rif_text(
        r#"
        app ActivityLog

        things:
          thing Profile
            field events: List<Text>
            field index: Int
            field next_event: Text
            field selected: Text
            field first: Text

        intent UpdateActivity

        subject:
          profile: Profile

        requires:
          profile.events[0] is created

        steps:
          1. Read indexed event
             set: profile.selected = profile.events[profile.index]

          2. Replace first event
             set: profile.events[0] = profile.next_event

          3. Read replaced event
             set: profile.first = profile.events[0]
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());
    let state: std::collections::BTreeMap<String, String> = [
        ("profile.events".to_string(), "[created,login]".to_string()),
        ("profile.index".to_string(), "1".to_string()),
        ("profile.next_event".to_string(), "logout".to_string()),
        ("profile.selected".to_string(), "".to_string()),
        ("profile.first".to_string(), "".to_string()),
    ]
    .into();

    let result = run_bytecode(
        &lower_document(&document),
        state.clone(),
        Default::default(),
    );
    assert_eq!(result.status, "succeeded");
    assert_eq!(result.final_state["profile.selected"], "login");
    assert_eq!(result.final_state["profile.events"], "[logout,login]");
    assert_eq!(result.final_state["profile.first"], "logout");
    assert!(!result.final_state.contains_key("profile.events.0"));

    let simulation = simulate(&document, state, Default::default());
    assert_eq!(simulation.status, "succeeded");
    assert_eq!(simulation.final_state["profile.selected"], "login");
    assert_eq!(simulation.final_state["profile.events"], "[logout,login]");
    assert_eq!(simulation.final_state["profile.first"], "logout");
    assert!(!simulation.final_state.contains_key("profile.events.0"));
}

#[test]
fn list_iteration_drives_runtime_flow() {
    let document = parse_rif_text(
        r#"
        app ActivityLog

        things:
          thing Profile
            field events: List<Text>
            field seen: List<Text>
            field current: Text

        intent ReplayActivity

        subject:
          profile: Profile

        steps:
          1. Replay events
             for each: profile.events as event
             set: profile.current = event
             append: profile.seen += event
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());
    let state: std::collections::BTreeMap<String, String> = [
        (
            "profile.events".to_string(),
            "[created,login,logout]".to_string(),
        ),
        ("profile.seen".to_string(), "[]".to_string()),
        ("profile.current".to_string(), "".to_string()),
    ]
    .into();

    let result = run_bytecode(
        &lower_document(&document),
        state.clone(),
        Default::default(),
    );
    assert_eq!(result.status, "succeeded");
    assert_eq!(result.final_state["profile.current"], "logout");
    assert_eq!(result.final_state["profile.seen"], "[created,login,logout]");

    let simulation = simulate(&document, state, Default::default());
    assert_eq!(simulation.status, "succeeded");
    assert_eq!(simulation.final_state["profile.current"], "logout");
    assert_eq!(
        simulation.final_state["profile.seen"],
        "[created,login,logout]"
    );
}

#[test]
fn checker_reports_invalid_list_append_element_types() {
    let document = parse_rif_text(
        r#"
        app BrokenAppend

        things:
          thing Batch
            field counts: List<Int>
            field label: Text

        intent AppendCount

        subject:
          batch: Batch

        steps:
          1. Append label
             append: batch.counts += batch.label
        "#,
    )
    .unwrap();

    assert!(
        check_document(&document)
            .iter()
            .any(|diagnostic| diagnostic.code == "EIGL_APPEND_VALUE_TYPE")
    );
}

#[test]
fn checker_accepts_map_literals_and_preserves_call_argument_commas() {
    let document = parse_rif_text(
        r#"
        app Dashboards

        things:
          thing Dashboard
            field counts: Map<Text, Int>

        operations:
          operation Metrics.publish(counts: Map<Text, Int>) -> Unit

        intent PublishDashboard

        subject:
          dashboard: Dashboard

        steps:
          1. Set counts
             set: dashboard.counts = {"open":1,"closed":2}
             changes: dashboard.counts

          2. Publish counts
             call: Metrics.publish({"open":1,"closed":2})

        guarantees:
          if this intent succeeds:
            dashboard exists
        "#,
    )
    .unwrap();

    assert_eq!(
        document.intent.steps[1].call.as_ref().unwrap().args,
        vec!["{\"open\":1,\"closed\":2}"]
    );
    assert_eq!(check_document(&document), Vec::new());
}

#[test]
fn map_lookup_expressions_drive_runtime_flow() {
    let document = parse_rif_text(
        r#"
        app Dashboards

        things:
          thing Dashboard
            field counts: Map<Text, Int>
            field status: Text
            field selected: Int
            field open_count: Int

        intent ReadDashboard

        subject:
          dashboard: Dashboard

        requires:
          dashboard.counts["open"] is 1

        steps:
          1. Read literal key
             set: dashboard.open_count = dashboard.counts["open"]

          2. Read live key
             set: dashboard.selected = dashboard.counts[dashboard.status]
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());
    let state: std::collections::BTreeMap<String, String> = [
        (
            "dashboard.counts".to_string(),
            "{\"open\":1,\"closed\":2}".to_string(),
        ),
        ("dashboard.status".to_string(), "closed".to_string()),
        ("dashboard.open_count".to_string(), "0".to_string()),
        ("dashboard.selected".to_string(), "0".to_string()),
    ]
    .into();

    let result = run_bytecode(
        &lower_document(&document),
        state.clone(),
        Default::default(),
    );
    assert_eq!(result.status, "succeeded");
    assert_eq!(result.final_state["dashboard.open_count"], "1");
    assert_eq!(result.final_state["dashboard.selected"], "2");

    let simulation = simulate(&document, state, Default::default());
    assert_eq!(simulation.status, "succeeded");
    assert_eq!(simulation.final_state["dashboard.open_count"], "1");
    assert_eq!(simulation.final_state["dashboard.selected"], "2");
}

#[test]
fn map_object_value_fields_are_typed_and_resolved() {
    let document = parse_rif_text(
        r#"
        app CommentDirectory

        things:
          thing Comment
            field id: Text
            field text: Text

          thing Directory
            field comments: Map<Text, Comment>
            field primary_text: Text

        intent ReviewPrimaryComment

        subject:
          directory: Directory

        steps:
          1. Copy primary comment text
             when: directory.comments["primary"].text contains "First"
             set: directory.primary_text = directory.comments["primary"].text
             changes: directory.primary_text

          2. Update primary comment text
             set: directory.comments["primary"].text = "Reviewed"
             changes: directory.comments

        returns:
          primary_text: directory.comments["primary"].text
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());

    let state: std::collections::BTreeMap<String, String> = [(
        "directory.comments".to_string(),
        r#"{"primary":{"id":"C1","text":"First"}}"#.to_string(),
    )]
    .into();

    let result = run_bytecode(
        &lower_document(&document),
        state.clone(),
        Default::default(),
    );
    assert_eq!(result.final_state["directory.primary_text"], "First");
    assert_eq!(
        result.final_state["directory.comments"],
        r#"{"primary":{"id":"C1","text":"Reviewed"}}"#
    );
    assert_eq!(result.outputs["primary_text"], "Reviewed");
    assert!(
        !result
            .final_state
            .contains_key(r#"directory.comments["primary"].text"#)
    );

    let simulation = simulate(&document, state, Default::default());
    assert_eq!(simulation.final_state["directory.primary_text"], "First");
    assert_eq!(
        simulation.final_state["directory.comments"],
        r#"{"primary":{"id":"C1","text":"Reviewed"}}"#
    );
    assert_eq!(simulation.outputs["primary_text"], "Reviewed");
    assert!(
        !simulation
            .final_state
            .contains_key(r#"directory.comments["primary"].text"#)
    );
}

#[test]
fn map_iteration_drives_runtime_flow() {
    let document = parse_rif_text(
        r#"
        app Dashboards

        things:
          thing Dashboard
            field counts: Map<Text, Int>
            field statuses: List<Text>
            field total: Int
            field current_count: Int

        intent SummarizeDashboard

        subject:
          dashboard: Dashboard

        steps:
          1. Summarize counts
             for each: dashboard.counts as status
             append: dashboard.statuses += status
             set: dashboard.current_count = dashboard.counts[status]
             set: dashboard.total = dashboard.total + dashboard.counts[status]
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());
    let state: std::collections::BTreeMap<String, String> = [
        (
            "dashboard.counts".to_string(),
            "{\"open\":1,\"closed\":2}".to_string(),
        ),
        ("dashboard.statuses".to_string(), "[]".to_string()),
        ("dashboard.total".to_string(), "0".to_string()),
        ("dashboard.current_count".to_string(), "0".to_string()),
    ]
    .into();

    let result = run_bytecode(
        &lower_document(&document),
        state.clone(),
        Default::default(),
    );
    assert_eq!(result.status, "succeeded");
    assert_eq!(result.final_state["dashboard.statuses"], "[open,closed]");
    assert_eq!(result.final_state["dashboard.current_count"], "2");
    assert_eq!(result.final_state["dashboard.total"], "3");

    let simulation = simulate(&document, state, Default::default());
    assert_eq!(simulation.status, "succeeded");
    assert_eq!(
        simulation.final_state["dashboard.statuses"],
        "[open,closed]"
    );
    assert_eq!(simulation.final_state["dashboard.current_count"], "2");
    assert_eq!(simulation.final_state["dashboard.total"], "3");
}

#[test]
fn checker_reports_invalid_iteration_sources() {
    let document = parse_rif_text(
        r#"
        app BrokenIteration

        things:
          thing Ticket
            field id: Text
            field status: State<Open, Closed>

          thing Profile
            field name: Text
            field current: Text

        collections:
          collection tickets: Ticket

        intent IterateBrokenSources

        subject:
          profile: Profile

        steps:
          1. Iterate scalar
             for each: profile.name as letter
             set: profile.current = letter

          2. Iterate unknown
             for each: profile.missing as missing
             set: profile.current = missing

          3. Iterate bad collection filter
             for each: tickets[missing=Closed] as ticket_id
             set: profile.current = ticket_id
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_ITERATION_SOURCE_TYPE".to_string()));
    assert!(codes.contains(&"EIGL_UNKNOWN_ITERATION_SOURCE".to_string()));
    assert!(codes.contains(&"EIGL_UNKNOWN_COLLECTION_SELECTOR_FIELD".to_string()));
}

#[test]
fn checker_rejects_scalar_collection_iteration_sources() {
    let document = parse_rif_text(
        r#"
        app BrokenCollectionIteration

        things:
          thing Ticket
            field id: Text

          thing Report
            field value: Text

        collections:
          collection tickets: Ticket

        intent IterateScalarProjection

        subject:
          report: Report

        steps:
          1. Iterate count
             for each: tickets.count as count
             set: report.value = count
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_ITERATION_SOURCE_TYPE".to_string()));
}

#[test]
fn checker_reports_invalid_step_permission_targets() {
    let document = parse_rif_text(
        r#"
        app BrokenPermissions

        things:
          thing Invoice
            field subtotal: Int
            field total: Int

        intent CalculateInvoice

        subject:
          invoice: Invoice

        steps:
          1. Calculate total
             reads: invoice.missing
             changes: invoice.unknown
             set: invoice.total = invoice.subtotal
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_UNKNOWN_READ_TARGET".to_string()));
    assert!(codes.contains(&"EIGL_UNKNOWN_CHANGE_TARGET".to_string()));
}

#[test]
fn map_entry_assignments_update_runtime_map_values() {
    let document = parse_rif_text(
        r#"
        app Dashboards

        things:
          thing Dashboard
            field counts: Map<Text, Int>
            field status: Text
            field next_count: Int
            field selected: Int

        intent UpdateDashboard

        subject:
          dashboard: Dashboard

        steps:
          1. Update literal key
             set: dashboard.counts["open"] = 4

          2. Update live key
             set: dashboard.counts[dashboard.status] = dashboard.next_count

          3. Read updated key
             set: dashboard.selected = dashboard.counts[dashboard.status]
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());
    let state: std::collections::BTreeMap<String, String> = [
        (
            "dashboard.counts".to_string(),
            "{\"open\":1,\"closed\":2}".to_string(),
        ),
        ("dashboard.status".to_string(), "closed".to_string()),
        ("dashboard.next_count".to_string(), "5".to_string()),
        ("dashboard.selected".to_string(), "0".to_string()),
    ]
    .into();

    let result = run_bytecode(
        &lower_document(&document),
        state.clone(),
        Default::default(),
    );
    assert_eq!(result.status, "succeeded");
    assert_eq!(
        result.final_state["dashboard.counts"],
        "{\"open\":4,\"closed\":5}"
    );
    assert_eq!(result.final_state["dashboard.selected"], "5");
    assert!(!result.final_state.contains_key("dashboard.counts.open"));
    assert!(!result.final_state.contains_key("dashboard.counts.closed"));

    let simulation = simulate(&document, state, Default::default());
    assert_eq!(simulation.status, "succeeded");
    assert_eq!(
        simulation.final_state["dashboard.counts"],
        "{\"open\":4,\"closed\":5}"
    );
    assert_eq!(simulation.final_state["dashboard.selected"], "5");
    assert!(!simulation.final_state.contains_key("dashboard.counts.open"));
    assert!(
        !simulation
            .final_state
            .contains_key("dashboard.counts.closed")
    );
}

#[test]
fn container_entry_deletes_update_runtime_values() {
    let document = parse_rif_text(
        r#"
        app Containers

        things:
          thing Profile
            field events: List<Text>
            field index: Int
          thing Dashboard
            field counts: Map<Text, Int>
            field status: Text

        intent RemoveEntries

        subject:
          profile: Profile
          dashboard: Dashboard

        steps:
          1. Remove event
             delete: profile.events[profile.index]

          2. Remove count
             delete: dashboard.counts[dashboard.status]
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());
    let state: std::collections::BTreeMap<String, String> = [
        (
            "profile.events".to_string(),
            "[created,login,logout]".to_string(),
        ),
        ("profile.index".to_string(), "1".to_string()),
        (
            "dashboard.counts".to_string(),
            "{\"open\":1,\"closed\":2}".to_string(),
        ),
        ("dashboard.status".to_string(), "closed".to_string()),
    ]
    .into();

    let result = run_bytecode(
        &lower_document(&document),
        state.clone(),
        Default::default(),
    );
    assert_eq!(result.status, "succeeded");
    assert_eq!(result.final_state["profile.events"], "[created,logout]");
    assert_eq!(result.final_state["dashboard.counts"], "{\"open\":1}");
    assert!(!result.final_state.contains_key("profile.events.1"));
    assert!(!result.final_state.contains_key("dashboard.counts.closed"));

    let simulation = simulate(&document, state, Default::default());
    assert_eq!(simulation.status, "succeeded");
    assert_eq!(simulation.final_state["profile.events"], "[created,logout]");
    assert_eq!(simulation.final_state["dashboard.counts"], "{\"open\":1}");
    assert!(!simulation.final_state.contains_key("profile.events.1"));
    assert!(
        !simulation
            .final_state
            .contains_key("dashboard.counts.closed")
    );
}

#[test]
fn checker_reports_invalid_delete_targets() {
    let document = parse_rif_text(
        r#"
        app BrokenDeletes

        things:
          thing Profile
            field events: List<Text>
            field counts: Map<Text, Int>
            field name: Text
            field bad_index: Text
            field bad_key: Int

        intent RemoveBrokenEntries

        subject:
          profile: Profile

        steps:
          1. Delete list with text index
             delete: profile.events[profile.bad_index]

          2. Delete map with int key
             delete: profile.counts[profile.bad_key]

          3. Delete indexed scalar
             delete: profile.name[0]

          4. Delete unknown path
             delete: profile.missing[0]
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_DELETE_INDEX_TYPE".to_string()));
    assert!(codes.contains(&"EIGL_DELETE_KEY_TYPE".to_string()));
    assert!(codes.contains(&"EIGL_DELETE_TARGET_TYPE".to_string()));
    assert!(codes.contains(&"EIGL_UNKNOWN_DELETE_TARGET".to_string()));
}

#[test]
fn list_and_map_counts_drive_runtime_flow() {
    let document = parse_rif_text(
        r#"
        app ContainerCounts

        things:
          thing Profile
            field events: List<Text>
            field event_count: Int
          thing Dashboard
            field counts: Map<Text, Int>
            field bucket_count: Int

        intent CountContainers

        subject:
          profile: Profile
          dashboard: Dashboard

        requires:
          profile.events.count is 2
          dashboard.counts.count is 2

        steps:
          1. Count list
             set: profile.event_count = profile.events.count

          2. Count map
             set: dashboard.bucket_count = dashboard.counts.count
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());
    let state: std::collections::BTreeMap<String, String> = [
        ("profile.events".to_string(), "[created,login]".to_string()),
        ("profile.event_count".to_string(), "0".to_string()),
        (
            "dashboard.counts".to_string(),
            "{\"open\":1,\"closed\":2}".to_string(),
        ),
        ("dashboard.bucket_count".to_string(), "0".to_string()),
    ]
    .into();

    let result = run_bytecode(
        &lower_document(&document),
        state.clone(),
        Default::default(),
    );
    assert_eq!(result.status, "succeeded");
    assert_eq!(result.final_state["profile.event_count"], "2");
    assert_eq!(result.final_state["dashboard.bucket_count"], "2");

    let simulation = simulate(&document, state, Default::default());
    assert_eq!(simulation.status, "succeeded");
    assert_eq!(simulation.final_state["profile.event_count"], "2");
    assert_eq!(simulation.final_state["dashboard.bucket_count"], "2");
}

#[test]
fn checker_accepts_option_literals_in_assignments_and_operation_calls() {
    let document = parse_rif_text(
        r#"
        app Profiles

        things:
          thing User
            field nickname: Option<Text>
            field age: Option<Int>

        operations:
          operation Profile.update(nickname: Option<Text>, age: Option<Int>) -> Unit

        intent UpdateProfile

        subject:
          user: User

        steps:
          1. Clear nickname
             set: user.nickname = None
             changes: user.nickname

          2. Set age
             set: user.age = Some(42)
             changes: user.age

          3. Update profile
             call: Profile.update(Some("Ada"), None)

        guarantees:
          if this intent succeeds:
            user exists
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());
}

#[test]
fn option_value_projection_drives_runtime_flow() {
    let document = parse_rif_text(
        r#"
        app OptionalProfiles

        things:
          thing User
            field nickname: Option<Text>
            field display_name: Text

        intent NormalizeProfile

        subject:
          user: User

        steps:
          1. Copy current nickname
             when: user.nickname.value contains "Ada"
             set: user.display_name = user.nickname.value
             changes: user.display_name

          2. Rewrite nickname
             set: user.nickname.value = "Grace"
             changes: user.nickname

        returns:
          nickname: user.nickname.value
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());

    let state: std::collections::BTreeMap<String, String> =
        [("user.nickname".to_string(), r#"Some("Ada")"#.to_string())].into();

    let result = run_bytecode(
        &lower_document(&document),
        state.clone(),
        Default::default(),
    );
    assert_eq!(result.final_state["user.display_name"], "Ada");
    assert_eq!(result.final_state["user.nickname"], r#"Some("Grace")"#);
    assert_eq!(result.outputs["nickname"], "Grace");
    assert!(!result.final_state.contains_key("user.nickname.value"));

    let simulation = simulate(&document, state, Default::default());
    assert_eq!(simulation.final_state["user.display_name"], "Ada");
    assert_eq!(simulation.final_state["user.nickname"], r#"Some("Grace")"#);
    assert_eq!(simulation.outputs["nickname"], "Grace");
    assert!(!simulation.final_state.contains_key("user.nickname.value"));
}

#[test]
fn result_variant_projection_drives_runtime_flow() {
    let document = parse_rif_text(
        r#"
        app ResultPayments

        things:
          thing Payment
            field confirmation: Result<Text, Text>
            field rejection: Result<Text, Text>
            field confirmation_id: Text
            field rejection_reason: Text

        intent NormalizePayment

        subject:
          payment: Payment

        steps:
          1. Copy success confirmation
             when: payment.confirmation.success contains "auth"
             set: payment.confirmation_id = payment.confirmation.success
             changes: payment.confirmation_id

          2. Rewrite success confirmation
             set: payment.confirmation.success = "auth-456"
             changes: payment.confirmation

          3. Copy failure reason
             when: payment.rejection.failure contains "timeout"
             set: payment.rejection_reason = payment.rejection.failure
             changes: payment.rejection_reason

          4. Rewrite failure reason
             set: payment.rejection.failure = "declined"
             changes: payment.rejection

        returns:
          confirmation: payment.confirmation.success
          rejection: payment.rejection.failure
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());

    let state: std::collections::BTreeMap<String, String> = [
        (
            "payment.confirmation".to_string(),
            r#"Success("auth-123")"#.to_string(),
        ),
        (
            "payment.rejection".to_string(),
            r#"Failure("timeout")"#.to_string(),
        ),
    ]
    .into();

    let result = run_bytecode(
        &lower_document(&document),
        state.clone(),
        Default::default(),
    );
    assert_eq!(result.final_state["payment.confirmation_id"], "auth-123");
    assert_eq!(
        result.final_state["payment.confirmation"],
        r#"Success("auth-456")"#
    );
    assert_eq!(result.final_state["payment.rejection_reason"], "timeout");
    assert_eq!(
        result.final_state["payment.rejection"],
        r#"Failure("declined")"#
    );
    assert_eq!(result.outputs["confirmation"], "auth-456");
    assert_eq!(result.outputs["rejection"], "declined");
    assert!(
        !result
            .final_state
            .contains_key("payment.confirmation.success")
    );
    assert!(!result.final_state.contains_key("payment.rejection.failure"));

    let simulation = simulate(&document, state, Default::default());
    assert_eq!(
        simulation.final_state["payment.confirmation_id"],
        "auth-123"
    );
    assert_eq!(
        simulation.final_state["payment.confirmation"],
        r#"Success("auth-456")"#
    );
    assert_eq!(
        simulation.final_state["payment.rejection_reason"],
        "timeout"
    );
    assert_eq!(
        simulation.final_state["payment.rejection"],
        r#"Failure("declined")"#
    );
    assert_eq!(simulation.outputs["confirmation"], "auth-456");
    assert_eq!(simulation.outputs["rejection"], "declined");
    assert!(
        !simulation
            .final_state
            .contains_key("payment.confirmation.success")
    );
    assert!(
        !simulation
            .final_state
            .contains_key("payment.rejection.failure")
    );
}

#[test]
fn wrapper_object_payload_fields_can_be_updated() {
    let document = parse_rif_text(
        r#"
        app ReviewWorkflow

        things:
          thing Review
            field text: Text
            field status: State<Draft, Published>

          thing Article
            field review: Option<Review>
            field moderation: Result<Review, Text>
            field summary: Text

        intent PublishArticle

        subject:
          article: Article

        steps:
          1. Copy review text
             when: article.review.value.text contains "Draft"
             set: article.summary = article.review.value.text
             changes: article.summary

          2. Rewrite review text
             set: article.review.value.text = "Final copy"
             changes: article.review

          3. Publish moderation result
             when: article.moderation.success.status is Draft
             set: article.moderation.success.status = Published
             changes: article.moderation

        returns:
          review_text: article.review.value.text
          moderation_status: article.moderation.success.status
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());

    let state: std::collections::BTreeMap<String, String> = [
        (
            "article.review".to_string(),
            r#"Some({"text":"Draft copy","status":Draft})"#.to_string(),
        ),
        (
            "article.moderation".to_string(),
            r#"Success({"text":"Needs review","status":Draft})"#.to_string(),
        ),
    ]
    .into();

    let result = run_bytecode(
        &lower_document(&document),
        state.clone(),
        Default::default(),
    );
    assert_eq!(result.final_state["article.summary"], "Draft copy");
    assert_eq!(
        result.final_state["article.review"],
        r#"Some({"text":"Final copy","status":Draft})"#
    );
    assert_eq!(
        result.final_state["article.moderation"],
        r#"Success({"text":"Needs review","status":"Published"})"#
    );
    assert_eq!(result.outputs["review_text"], "Final copy");
    assert_eq!(result.outputs["moderation_status"], "Published");
    assert!(!result.final_state.contains_key("article.review.value.text"));
    assert!(
        !result
            .final_state
            .contains_key("article.moderation.success.status")
    );

    let simulation = simulate(&document, state, Default::default());
    assert_eq!(simulation.final_state["article.summary"], "Draft copy");
    assert_eq!(
        simulation.final_state["article.review"],
        r#"Some({"text":"Final copy","status":Draft})"#
    );
    assert_eq!(
        simulation.final_state["article.moderation"],
        r#"Success({"text":"Needs review","status":"Published"})"#
    );
    assert_eq!(simulation.outputs["review_text"], "Final copy");
    assert_eq!(simulation.outputs["moderation_status"], "Published");
    assert!(
        !simulation
            .final_state
            .contains_key("article.review.value.text")
    );
    assert!(
        !simulation
            .final_state
            .contains_key("article.moderation.success.status")
    );
}

#[test]
fn wrapper_nested_object_payload_fields_can_be_updated() {
    let document = parse_rif_text(
        r#"
        app NestedReviewWorkflow

        things:
          thing ReviewDetails
            field note: Text
            field visibility: State<Private, Public>

          thing Review
            field text: Text
            field details: ReviewDetails

          thing Article
            field review: Option<Review>
            field moderation: Result<Review, Text>

        intent PublishNestedReview

        subject:
          article: Article

        steps:
          1. Rewrite nested review note
             when: article.review.value.details.note contains "Draft"
             set: article.review.value.details.note = "Final note"
             changes: article.review

          2. Publish nested moderation detail
             when: article.moderation.success.details.visibility is Private
             set: article.moderation.success.details.visibility = Public
             changes: article.moderation

        returns:
          note: article.review.value.details.note
          visibility: article.moderation.success.details.visibility
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());

    let state: std::collections::BTreeMap<String, String> = [
        (
            "article.review".to_string(),
            r#"Some({"text":"Draft copy","details":{"note":"Draft note","visibility":Private}})"#
                .to_string(),
        ),
        (
            "article.moderation".to_string(),
            r#"Success({"text":"Needs review","details":{"note":"Moderation note","visibility":Private}})"#
                .to_string(),
        ),
    ]
    .into();

    let result = run_bytecode(
        &lower_document(&document),
        state.clone(),
        Default::default(),
    );
    assert_eq!(
        result.final_state["article.review"],
        r#"Some({"text":"Draft copy","details":{"note":"Final note","visibility":Private}})"#
    );
    assert_eq!(
        result.final_state["article.moderation"],
        r#"Success({"text":"Needs review","details":{"note":"Moderation note","visibility":"Public"}})"#
    );
    assert_eq!(result.outputs["note"], "Final note");
    assert_eq!(result.outputs["visibility"], "Public");
    assert!(
        !result
            .final_state
            .contains_key("article.review.value.details.note")
    );
    assert!(
        !result
            .final_state
            .contains_key("article.moderation.success.details.visibility")
    );

    let simulation = simulate(&document, state, Default::default());
    assert_eq!(
        simulation.final_state["article.review"],
        r#"Some({"text":"Draft copy","details":{"note":"Final note","visibility":Private}})"#
    );
    assert_eq!(
        simulation.final_state["article.moderation"],
        r#"Success({"text":"Needs review","details":{"note":"Moderation note","visibility":"Public"}})"#
    );
    assert_eq!(simulation.outputs["note"], "Final note");
    assert_eq!(simulation.outputs["visibility"], "Public");
    assert!(
        !simulation
            .final_state
            .contains_key("article.review.value.details.note")
    );
    assert!(
        !simulation
            .final_state
            .contains_key("article.moderation.success.details.visibility")
    );
}

#[test]
fn wrapper_container_object_payload_fields_can_be_updated() {
    let document = parse_rif_text(
        r#"
        app WrappedCommentWorkflow

        things:
          thing Comment
            field id: Text
            field text: Text

          thing Thread
            field comments: Option<List<Comment>>
            field directory: Result<Map<Text, Comment>, Text>
            field first_text: Text
            field primary_text: Text

        intent ReviewWrappedComments

        subject:
          thread: Thread

        steps:
          1. Copy first optional comment
             when: thread.comments.value[0].text contains "First"
             set: thread.first_text = thread.comments.value[0].text
             changes: thread.first_text

          2. Update first optional comment
             set: thread.comments.value[0].text = "Reviewed"
             changes: thread.comments

          3. Copy primary result comment
             when: thread.directory.success["primary"].text contains "Draft"
             set: thread.primary_text = thread.directory.success["primary"].text
             changes: thread.primary_text

          4. Update primary result comment
             set: thread.directory.success["primary"].text = "Published"
             changes: thread.directory

        returns:
          first_text: thread.comments.value[0].text
          primary_text: thread.directory.success["primary"].text
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());

    let state: std::collections::BTreeMap<String, String> = [
        (
            "thread.comments".to_string(),
            r#"Some([{"id":"C1","text":"First draft"}])"#.to_string(),
        ),
        (
            "thread.directory".to_string(),
            r#"Success({"primary":{"id":"C2","text":"Draft primary"}})"#.to_string(),
        ),
    ]
    .into();

    let result = run_bytecode(
        &lower_document(&document),
        state.clone(),
        Default::default(),
    );
    assert_eq!(result.final_state["thread.first_text"], "First draft");
    assert_eq!(
        result.final_state["thread.comments"],
        r#"Some([{"id":"C1","text":"Reviewed"}])"#
    );
    assert_eq!(result.final_state["thread.primary_text"], "Draft primary");
    assert_eq!(
        result.final_state["thread.directory"],
        r#"Success({"primary":{"id":"C2","text":"Published"}})"#
    );
    assert_eq!(result.outputs["first_text"], "Reviewed");
    assert_eq!(result.outputs["primary_text"], "Published");
    assert!(
        !result
            .final_state
            .contains_key("thread.comments.value[0].text")
    );
    assert!(
        !result
            .final_state
            .contains_key(r#"thread.directory.success["primary"].text"#)
    );

    let simulation = simulate(&document, state, Default::default());
    assert_eq!(simulation.final_state["thread.first_text"], "First draft");
    assert_eq!(
        simulation.final_state["thread.comments"],
        r#"Some([{"id":"C1","text":"Reviewed"}])"#
    );
    assert_eq!(
        simulation.final_state["thread.primary_text"],
        "Draft primary"
    );
    assert_eq!(
        simulation.final_state["thread.directory"],
        r#"Success({"primary":{"id":"C2","text":"Published"}})"#
    );
    assert_eq!(simulation.outputs["first_text"], "Reviewed");
    assert_eq!(simulation.outputs["primary_text"], "Published");
    assert!(
        !simulation
            .final_state
            .contains_key("thread.comments.value[0].text")
    );
    assert!(
        !simulation
            .final_state
            .contains_key(r#"thread.directory.success["primary"].text"#)
    );
}

#[test]
fn wrapper_container_entries_can_be_updated() {
    let document = parse_rif_text(
        r#"
        app WrappedContainerEntries

        things:
          thing Profile
            field tags: Option<List<Text>>
            field counts: Result<Map<Text, Int>, Text>
            field status: Text
            field next_count: Int
            field selected_tag: Text
            field selected_count: Int

        intent UpdateWrappedContainerEntries

        subject:
          profile: Profile

        steps:
          1. Copy optional tag
             when: profile.tags.value[0] contains "draft"
             set: profile.selected_tag = profile.tags.value[0]
             changes: profile.selected_tag

          2. Update optional tag
             set: profile.tags.value[0] = "published"
             changes: profile.tags

          3. Update literal result map key
             set: profile.counts.success["open"] = 4
             changes: profile.counts

          4. Update live result map key
             set: profile.counts.success[profile.status] = profile.next_count
             changes: profile.counts

          5. Read updated result map key
             set: profile.selected_count = profile.counts.success[profile.status]
             changes: profile.selected_count

        returns:
          tag: profile.tags.value[0]
          count: profile.counts.success[profile.status]
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());

    let state: std::collections::BTreeMap<String, String> = [
        (
            "profile.tags".to_string(),
            "Some([draft,archived])".to_string(),
        ),
        (
            "profile.counts".to_string(),
            r#"Success({"open":1,"closed":2})"#.to_string(),
        ),
        ("profile.status".to_string(), "closed".to_string()),
        ("profile.next_count".to_string(), "5".to_string()),
    ]
    .into();

    let result = run_bytecode(
        &lower_document(&document),
        state.clone(),
        Default::default(),
    );
    assert_eq!(result.final_state["profile.selected_tag"], "draft");
    assert_eq!(
        result.final_state["profile.tags"],
        "Some([published,archived])"
    );
    assert_eq!(
        result.final_state["profile.counts"],
        r#"Success({"open":4,"closed":5})"#
    );
    assert_eq!(result.final_state["profile.selected_count"], "5");
    assert_eq!(result.outputs["tag"], "published");
    assert_eq!(result.outputs["count"], "5");
    assert!(!result.final_state.contains_key("profile.tags.value[0]"));
    assert!(
        !result
            .final_state
            .contains_key(r#"profile.counts.success["open"]"#)
    );
    assert!(
        !result
            .final_state
            .contains_key("profile.counts.success[profile.status]")
    );

    let simulation = simulate(&document, state, Default::default());
    assert_eq!(simulation.final_state["profile.selected_tag"], "draft");
    assert_eq!(
        simulation.final_state["profile.tags"],
        "Some([published,archived])"
    );
    assert_eq!(
        simulation.final_state["profile.counts"],
        r#"Success({"open":4,"closed":5})"#
    );
    assert_eq!(simulation.final_state["profile.selected_count"], "5");
    assert_eq!(simulation.outputs["tag"], "published");
    assert_eq!(simulation.outputs["count"], "5");
    assert!(!simulation.final_state.contains_key("profile.tags.value[0]"));
    assert!(
        !simulation
            .final_state
            .contains_key(r#"profile.counts.success["open"]"#)
    );
    assert!(
        !simulation
            .final_state
            .contains_key("profile.counts.success[profile.status]")
    );
}

#[test]
fn wrapper_containers_can_append_and_delete_entries() {
    let document = parse_rif_text(
        r#"
        app WrappedContainerMutations

        things:
          thing Profile
            field tags: Option<List<Text>>
            field audit: Result<List<Text>, Text>
            field counts: Result<Map<Text, Int>, Text>
            field next_tag: Text
            field next_event: Text

        intent MutateWrappedContainers

        subject:
          profile: Profile

        steps:
          1. Append optional tag
             append: profile.tags.value += profile.next_tag
             changes: profile.tags

          2. Delete archived optional tag
             delete: profile.tags.value[1]
             changes: profile.tags

          3. Append result audit entry
             append: profile.audit.success += profile.next_event
             changes: profile.audit

          4. Delete closed count
             delete: profile.counts.success["closed"]
             changes: profile.counts

        returns:
          first_tag: profile.tags.value[0]
          last_audit: profile.audit.success[1]
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());

    let state: std::collections::BTreeMap<String, String> = [
        (
            "profile.tags".to_string(),
            "Some([draft,archived])".to_string(),
        ),
        (
            "profile.audit".to_string(),
            "Success([created])".to_string(),
        ),
        (
            "profile.counts".to_string(),
            r#"Success({"open":1,"closed":2})"#.to_string(),
        ),
        ("profile.next_tag".to_string(), "published".to_string()),
        ("profile.next_event".to_string(), "reviewed".to_string()),
    ]
    .into();

    let result = run_bytecode(
        &lower_document(&document),
        state.clone(),
        Default::default(),
    );
    assert_eq!(
        result.final_state["profile.tags"],
        "Some([draft,published])"
    );
    assert_eq!(
        result.final_state["profile.audit"],
        "Success([created,reviewed])"
    );
    assert_eq!(
        result.final_state["profile.counts"],
        r#"Success({"open":1})"#
    );
    assert_eq!(result.outputs["first_tag"], "draft");
    assert_eq!(result.outputs["last_audit"], "reviewed");
    assert!(!result.final_state.contains_key("profile.tags.value"));
    assert!(!result.final_state.contains_key("profile.tags.value[1]"));
    assert!(!result.final_state.contains_key("profile.audit.success"));
    assert!(
        !result
            .final_state
            .contains_key(r#"profile.counts.success["closed"]"#)
    );

    let simulation = simulate(&document, state, Default::default());
    assert_eq!(
        simulation.final_state["profile.tags"],
        "Some([draft,published])"
    );
    assert_eq!(
        simulation.final_state["profile.audit"],
        "Success([created,reviewed])"
    );
    assert_eq!(
        simulation.final_state["profile.counts"],
        r#"Success({"open":1})"#
    );
    assert_eq!(simulation.outputs["first_tag"], "draft");
    assert_eq!(simulation.outputs["last_audit"], "reviewed");
    assert!(!simulation.final_state.contains_key("profile.tags.value"));
    assert!(!simulation.final_state.contains_key("profile.tags.value[1]"));
    assert!(!simulation.final_state.contains_key("profile.audit.success"));
    assert!(
        !simulation
            .final_state
            .contains_key(r#"profile.counts.success["closed"]"#)
    );
}

#[test]
fn container_held_wrapper_entries_can_be_updated() {
    let document = parse_rif_text(
        r#"
        app ContainerHeldWrappers

        things:
          thing Profile
            field aliases: List<Option<Text>>
            field checks: Map<Text, Result<Int, Text>>
            field next_alias: Text
            field next_score: Int
            field selected_alias: Text
            field selected_score: Int

        intent UpdateContainerHeldWrappers

        subject:
          profile: Profile

        steps:
          1. Read first alias
             when: profile.aliases[0].value contains "draft"
             set: profile.selected_alias = profile.aliases[0].value
             changes: profile.selected_alias

          2. Fill missing alias
             set: profile.aliases[1].value = profile.next_alias
             changes: profile.aliases

          3. Update open check
             set: profile.checks["open"].success = profile.next_score
             changes: profile.checks

          4. Mark blocked check
             set: profile.checks["blocked"].failure = "manual"
             changes: profile.checks

          5. Read updated check
             set: profile.selected_score = profile.checks["open"].success
             changes: profile.selected_score

        returns:
          alias: profile.aliases[1].value
          score: profile.checks["open"].success
          failure: profile.checks["blocked"].failure
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());

    let state: std::collections::BTreeMap<String, String> = [
        (
            "profile.aliases".to_string(),
            r#"[Some("draft"),None]"#.to_string(),
        ),
        (
            "profile.checks".to_string(),
            r#"{"open":Success(1),"blocked":Success(2)}"#.to_string(),
        ),
        ("profile.next_alias".to_string(), "published".to_string()),
        ("profile.next_score".to_string(), "4".to_string()),
    ]
    .into();

    let result = run_bytecode(
        &lower_document(&document),
        state.clone(),
        Default::default(),
    );
    assert_eq!(result.final_state["profile.selected_alias"], "draft");
    assert_eq!(
        result.final_state["profile.aliases"],
        r#"[Some("draft"),Some("published")]"#
    );
    assert_eq!(
        result.final_state["profile.checks"],
        r#"{"open":Success(4),"blocked":Failure("manual")}"#
    );
    assert_eq!(result.final_state["profile.selected_score"], "4");
    assert_eq!(result.outputs["alias"], "published");
    assert_eq!(result.outputs["score"], "4");
    assert_eq!(result.outputs["failure"], "manual");
    assert!(!result.final_state.contains_key("profile.aliases.1.value"));
    assert!(
        !result
            .final_state
            .contains_key(r#"profile.checks.open.success"#)
    );

    let simulation = simulate(&document, state, Default::default());
    assert_eq!(simulation.final_state["profile.selected_alias"], "draft");
    assert_eq!(
        simulation.final_state["profile.aliases"],
        r#"[Some("draft"),Some("published")]"#
    );
    assert_eq!(
        simulation.final_state["profile.checks"],
        r#"{"open":Success(4),"blocked":Failure("manual")}"#
    );
    assert_eq!(simulation.final_state["profile.selected_score"], "4");
    assert_eq!(simulation.outputs["alias"], "published");
    assert_eq!(simulation.outputs["score"], "4");
    assert_eq!(simulation.outputs["failure"], "manual");
    assert!(
        !simulation
            .final_state
            .contains_key("profile.aliases.1.value")
    );
    assert!(
        !simulation
            .final_state
            .contains_key(r#"profile.checks.open.success"#)
    );
}

#[test]
fn container_held_wrapper_object_fields_can_be_updated() {
    let document = parse_rif_text(
        r#"
        app ContainerHeldWrapperObjects

        things:
          thing Comment
            field id: Text
            field text: Text

          thing Thread
            field comments: List<Option<Comment>>
            field directory: Map<Text, Result<Comment, Text>>
            field first_text: Text
            field primary_text: Text

        intent ReviewContainerHeldWrappers

        subject:
          thread: Thread

        steps:
          1. Copy optional comment text
             when: thread.comments[0].value.text contains "Draft"
             set: thread.first_text = thread.comments[0].value.text
             changes: thread.first_text

          2. Update optional comment text
             set: thread.comments[0].value.text = "Reviewed"
             changes: thread.comments

          3. Copy result comment text
             when: thread.directory["primary"].success.text contains "Queued"
             set: thread.primary_text = thread.directory["primary"].success.text
             changes: thread.primary_text

          4. Update result comment text
             set: thread.directory["primary"].success.text = "Published"
             changes: thread.directory

        returns:
          optional_text: thread.comments[0].value.text
          result_text: thread.directory["primary"].success.text
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());

    let state: std::collections::BTreeMap<String, String> = [
        (
            "thread.comments".to_string(),
            r#"[Some({"id":"C1","text":"Draft comment"}),None]"#.to_string(),
        ),
        (
            "thread.directory".to_string(),
            r#"{"primary":Success({"id":"C2","text":"Queued comment"})}"#.to_string(),
        ),
    ]
    .into();

    let result = run_bytecode(
        &lower_document(&document),
        state.clone(),
        Default::default(),
    );
    assert_eq!(result.final_state["thread.first_text"], "Draft comment");
    assert_eq!(
        result.final_state["thread.comments"],
        r#"[Some({"id":"C1","text":"Reviewed"}),None]"#
    );
    assert_eq!(result.final_state["thread.primary_text"], "Queued comment");
    assert_eq!(
        result.final_state["thread.directory"],
        r#"{"primary":Success({"id":"C2","text":"Published"})}"#
    );
    assert_eq!(result.outputs["optional_text"], "Reviewed");
    assert_eq!(result.outputs["result_text"], "Published");
    assert!(
        !result
            .final_state
            .contains_key("thread.comments.0.value.text")
    );
    assert!(
        !result
            .final_state
            .contains_key(r#"thread.directory.primary.success.text"#)
    );

    let simulation = simulate(&document, state, Default::default());
    assert_eq!(simulation.final_state["thread.first_text"], "Draft comment");
    assert_eq!(
        simulation.final_state["thread.comments"],
        r#"[Some({"id":"C1","text":"Reviewed"}),None]"#
    );
    assert_eq!(
        simulation.final_state["thread.primary_text"],
        "Queued comment"
    );
    assert_eq!(
        simulation.final_state["thread.directory"],
        r#"{"primary":Success({"id":"C2","text":"Published"})}"#
    );
    assert_eq!(simulation.outputs["optional_text"], "Reviewed");
    assert_eq!(simulation.outputs["result_text"], "Published");
    assert!(
        !simulation
            .final_state
            .contains_key("thread.comments.0.value.text")
    );
    assert!(
        !simulation
            .final_state
            .contains_key(r#"thread.directory.primary.success.text"#)
    );
}

#[test]
fn container_held_wrapper_containers_can_append_and_delete_entries() {
    let document = parse_rif_text(
        r#"
        app ContainerHeldWrapperContainers

        things:
          thing Profile
            field tag_groups: List<Option<List<Text>>>
            field audit_groups: Map<Text, Result<List<Text>, Text>>
            field counts_by_status: Map<Text, Result<Map<Text, Int>, Text>>
            field next_tag: Text
            field next_event: Text

        intent MutateContainerHeldWrapperContainers

        subject:
          profile: Profile

        steps:
          1. Append nested optional tag
             append: profile.tag_groups[0].value += profile.next_tag
             changes: profile.tag_groups

          2. Delete nested archived optional tag
             delete: profile.tag_groups[0].value[1]
             changes: profile.tag_groups

          3. Append nested result audit entry
             append: profile.audit_groups["main"].success += profile.next_event
             changes: profile.audit_groups

          4. Delete nested result count
             delete: profile.counts_by_status["current"].success["closed"]
             changes: profile.counts_by_status

        returns:
          tag: profile.tag_groups[0].value[1]
          audit: profile.audit_groups["main"].success[1]
          open_count: profile.counts_by_status["current"].success["open"]
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());

    let state: std::collections::BTreeMap<String, String> = [
        (
            "profile.tag_groups".to_string(),
            "[Some([draft,archived]),None]".to_string(),
        ),
        (
            "profile.audit_groups".to_string(),
            r#"{"main":Success([created])}"#.to_string(),
        ),
        (
            "profile.counts_by_status".to_string(),
            r#"{"current":Success({"open":1,"closed":2})}"#.to_string(),
        ),
        ("profile.next_tag".to_string(), "published".to_string()),
        ("profile.next_event".to_string(), "reviewed".to_string()),
    ]
    .into();

    let result = run_bytecode(
        &lower_document(&document),
        state.clone(),
        Default::default(),
    );
    assert_eq!(
        result.final_state["profile.tag_groups"],
        "[Some([draft,published]),None]"
    );
    assert_eq!(
        result.final_state["profile.audit_groups"],
        r#"{"main":Success([created,reviewed])}"#
    );
    assert_eq!(
        result.final_state["profile.counts_by_status"],
        r#"{"current":Success({"open":1})}"#
    );
    assert_eq!(result.outputs["tag"], "published");
    assert_eq!(result.outputs["audit"], "reviewed");
    assert_eq!(result.outputs["open_count"], "1");
    assert!(
        !result
            .final_state
            .contains_key("profile.tag_groups.0.value")
    );
    assert!(
        !result
            .final_state
            .contains_key(r#"profile.audit_groups.main.success"#)
    );
    assert!(
        !result
            .final_state
            .contains_key(r#"profile.counts_by_status.current.success["closed"]"#)
    );

    let simulation = simulate(&document, state, Default::default());
    assert_eq!(
        simulation.final_state["profile.tag_groups"],
        "[Some([draft,published]),None]"
    );
    assert_eq!(
        simulation.final_state["profile.audit_groups"],
        r#"{"main":Success([created,reviewed])}"#
    );
    assert_eq!(
        simulation.final_state["profile.counts_by_status"],
        r#"{"current":Success({"open":1})}"#
    );
    assert_eq!(simulation.outputs["tag"], "published");
    assert_eq!(simulation.outputs["audit"], "reviewed");
    assert_eq!(simulation.outputs["open_count"], "1");
    assert!(
        !simulation
            .final_state
            .contains_key("profile.tag_groups.0.value")
    );
    assert!(
        !simulation
            .final_state
            .contains_key(r#"profile.audit_groups.main.success"#)
    );
    assert!(
        !simulation
            .final_state
            .contains_key(r#"profile.counts_by_status.current.success["closed"]"#)
    );
}

#[test]
fn checker_accepts_result_literals_in_assignments_and_operation_calls() {
    let document = parse_rif_text(
        r#"
        app Payments

        things:
          thing Payment
            field confirmation: Result<Text, Text>

        operations:
          operation Payment.record(result: Result<Text, Text>) -> Unit

        intent CapturePayment

        subject:
          payment: Payment

        steps:
          1. Store success
             set: payment.confirmation = Success("auth-123")
             changes: payment.confirmation

          2. Record failure case
             call: Payment.record(Failure("timeout"))

        guarantees:
          if this intent succeeds:
            payment exists
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());
}

#[test]
fn parser_and_checker_support_user_defined_enums() {
    let document = parse_rif_text(
        r#"
        app TriageDesk

        types:
          enum Priority
            value Low
            value Normal
            value Critical

        things:
          thing Ticket
            field priority: Priority

        operations:
          operation Routing.assign(priority: Priority) -> Unit

        intent EscalateTicket

        subject:
          ticket: Ticket

        steps:
          1. Mark priority
             set: ticket.priority = Critical
             changes: ticket.priority

          2. Assign route
             call: Routing.assign(Critical)

        guarantees:
          if this intent succeeds:
            ticket exists
        "#,
    )
    .unwrap();

    assert_eq!(
        document.application.enums["Priority"].values,
        vec!["Low", "Normal", "Critical"]
    );
    assert_eq!(check_document(&document), Vec::new());
}

#[test]
fn checker_requires_handling_operation_declared_failures() {
    let document = parse_rif_text(
        r#"
        app FailureContracts

        things:
          thing Job
            field id: Id<Job>

        operations:
          operation Worker.run(job: Job) -> Unit
            may fail with: WorkerFailed

        intent RunJob

        subject:
          job: Job

        steps:
          1. Run worker
             call: Worker.run(job)

        guarantees:
          if this intent succeeds:
            job exists
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_UNHANDLED_FAILURE".to_string()));
}

#[test]
fn guarded_steps_lower_to_conditional_bytecode_and_run_selectively() {
    let document = parse_rif_file(fixture("ticket_routing_app.rif.md")).unwrap();
    assert_eq!(check_document(&document), Vec::new());
    assert_eq!(
        document.intent.steps[0].guard.as_deref(),
        Some("ticket.priority is Critical")
    );
    assert_eq!(
        document.intent.steps[1].guard.as_deref(),
        Some("ticket.priority is not Critical")
    );

    let bytecode = lower_document(&document);
    assert!(
        bytecode
            .instructions
            .iter()
            .any(|instruction| instruction.opcode == "BEGIN_STEP"
                && instruction.fields["step"] == "Escalate critical ticket"
                && instruction.fields["guard"] == "ticket.priority is Critical")
    );
    let program = build_program(&document);
    assert!(program.graph.has_edge(
        "requires",
        "Escalate critical ticket",
        "ticket.priority is Critical"
    ));

    let critical = run_bytecode(
        &bytecode,
        [
            ("ticket.route".to_string(), "Unrouted".to_string()),
            ("ticket.priority".to_string(), "Critical".to_string()),
        ]
        .into(),
        [].into(),
    );
    assert_eq!(critical.status, "succeeded");
    assert_eq!(critical.final_state["ticket.route"], "Escalated");
    assert!(critical.outputs.contains_key("escalation"));
    assert!(!critical.outputs.contains_key("work_item"));
    assert!(
        critical
            .trace
            .contains(&"SKIP Queue normal ticket when ticket.priority is not Critical".to_string())
    );

    let normal = run_bytecode(
        &bytecode,
        [
            ("ticket.route".to_string(), "Unrouted".to_string()),
            ("ticket.priority".to_string(), "Normal".to_string()),
        ]
        .into(),
        [].into(),
    );
    assert_eq!(normal.status, "succeeded");
    assert_eq!(normal.final_state["ticket.route"], "Queued");
    assert!(normal.outputs.contains_key("work_item"));
    assert!(!normal.outputs.contains_key("escalation"));
    assert!(
        normal.trace.contains(
            &"SKIP Escalate critical ticket when ticket.priority is Critical".to_string()
        )
    );
}

#[test]
fn bytecode_vm_evaluates_compound_predicates_in_requirements_guards_and_guarantees() {
    let document = parse_rif_text(
        r#"
        app CompoundPredicates

        things:
          thing Invoice
            field status: State<Draft, Finalized>
            field subtotal: Int
            field tax: Int
            field discount: Int
            field total: Int

        intent FinalizeInvoice

        subject:
          invoice: Invoice

        requires:
          invoice.status is Draft and invoice.subtotal > 0

        steps:
          1. Set payable total
             when: invoice.tax > 0 or invoice.discount > 0
             set: invoice.total = 1
             changes: invoice.total

          2. Finalize invoice
             set: invoice.status = Finalized
             changes: invoice.status

        guarantees:
          if this intent succeeds:
            invoice.status is Finalized and invoice.total > 0
        "#,
    )
    .unwrap();

    let bytecode = lower_document(&document);
    let success = run_bytecode(
        &bytecode,
        [
            ("invoice.status".to_string(), "Draft".to_string()),
            ("invoice.subtotal".to_string(), "10".to_string()),
            ("invoice.tax".to_string(), "0".to_string()),
            ("invoice.discount".to_string(), "1".to_string()),
        ]
        .into(),
        [].into(),
    );
    assert_eq!(success.status, "succeeded");
    assert_eq!(success.final_state["invoice.total"], "1");
    assert!(
        success
            .trace
            .contains(&"ASSERT invoice.status is Finalized and invoice.total > 0".to_string())
    );

    let missing_subtotal = run_bytecode(
        &bytecode,
        [
            ("invoice.status".to_string(), "Draft".to_string()),
            ("invoice.subtotal".to_string(), "0".to_string()),
            ("invoice.tax".to_string(), "1".to_string()),
            ("invoice.discount".to_string(), "0".to_string()),
        ]
        .into(),
        [].into(),
    );
    assert_eq!(missing_subtotal.status, "failed");
    assert_eq!(
        missing_subtotal.failure.as_deref(),
        Some("RequirementFailed")
    );
    assert!(
        missing_subtotal
            .trace
            .contains(&"CHECK FAILED invoice.status is Draft and invoice.subtotal > 0".to_string())
    );

    let skipped_total = run_bytecode(
        &bytecode,
        [
            ("invoice.status".to_string(), "Draft".to_string()),
            ("invoice.subtotal".to_string(), "10".to_string()),
            ("invoice.tax".to_string(), "0".to_string()),
            ("invoice.discount".to_string(), "0".to_string()),
        ]
        .into(),
        [].into(),
    );
    assert_eq!(skipped_total.status, "failed");
    assert_eq!(skipped_total.failure.as_deref(), Some("GuaranteeFailed"));
    assert!(skipped_total.trace.contains(
        &"SKIP Set payable total when invoice.tax > 0 or invoice.discount > 0".to_string()
    ));
}

#[test]
fn bytecode_vm_evaluates_negated_and_grouped_predicates() {
    let document = parse_rif_text(
        r#"
        app GroupedPredicates

        things:
          thing Invoice
            field status: State<Draft, Finalized>
            field subtotal: Int
            field tax: Int
            field discount: Int
            field total: Int

        intent FinalizeInvoice

        subject:
          invoice: Invoice

        requires:
          not (invoice.status is Finalized) and (invoice.subtotal > 0 or invoice.tax > 0)

        steps:
          1. Set payable total
             when: not (invoice.tax > 0 and invoice.discount > 0)
             set: invoice.total = 1
             changes: invoice.total

          2. Finalize invoice
             set: invoice.status = Finalized
             changes: invoice.status

        guarantees:
          if this intent succeeds:
            not (invoice.status is Draft) and invoice.total > 0
        "#,
    )
    .unwrap();
    assert_eq!(check_document(&document), Vec::new());

    let bytecode = lower_document(&document);
    let success = run_bytecode(
        &bytecode,
        [
            ("invoice.status".to_string(), "Draft".to_string()),
            ("invoice.subtotal".to_string(), "0".to_string()),
            ("invoice.tax".to_string(), "1".to_string()),
            ("invoice.discount".to_string(), "0".to_string()),
        ]
        .into(),
        [].into(),
    );
    assert_eq!(success.status, "succeeded");
    assert_eq!(success.final_state["invoice.total"], "1");

    let blocked_status = run_bytecode(
        &bytecode,
        [
            ("invoice.status".to_string(), "Finalized".to_string()),
            ("invoice.subtotal".to_string(), "10".to_string()),
            ("invoice.tax".to_string(), "0".to_string()),
            ("invoice.discount".to_string(), "0".to_string()),
        ]
        .into(),
        [].into(),
    );
    assert_eq!(blocked_status.status, "failed");
    assert_eq!(blocked_status.failure.as_deref(), Some("RequirementFailed"));

    let skipped_total = run_bytecode(
        &bytecode,
        [
            ("invoice.status".to_string(), "Draft".to_string()),
            ("invoice.subtotal".to_string(), "10".to_string()),
            ("invoice.tax".to_string(), "1".to_string()),
            ("invoice.discount".to_string(), "1".to_string()),
        ]
        .into(),
        [].into(),
    );
    assert_eq!(skipped_total.status, "failed");
    assert_eq!(skipped_total.failure.as_deref(), Some("GuaranteeFailed"));
    assert!(skipped_total.trace.contains(
        &"SKIP Set payable total when not (invoice.tax > 0 and invoice.discount > 0)".to_string()
    ));
}

#[test]
fn containment_predicates_drive_runtime_flow() {
    let document = parse_rif_text(
        r#"
        app AccessControl

        things:
          thing Account
            field email: Text
            field roles: List<Text>
            field flags: Map<Text, Bool>
            field status: State<Pending, Allowed>

        intent AllowAccount

        subject:
          account: Account

        requires:
          account.email contains "@"
          account.roles contains "admin"
          account.flags contains "beta"

        steps:
          1. Allow matching account
             when: account.email contains "example.com" and account.roles contains "admin" and account.flags contains "beta"
             set: account.status = Allowed
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());
    let state: std::collections::BTreeMap<String, String> = [
        ("account.email".to_string(), "ada@example.com".to_string()),
        ("account.roles".to_string(), "[user,admin]".to_string()),
        (
            "account.flags".to_string(),
            "{\"beta\":true,\"staff\":false}".to_string(),
        ),
        ("account.status".to_string(), "Pending".to_string()),
    ]
    .into();

    let result = run_bytecode(
        &lower_document(&document),
        state.clone(),
        Default::default(),
    );
    assert_eq!(result.status, "succeeded");
    assert_eq!(result.final_state["account.status"], "Allowed");

    let simulation = simulate(&document, state, Default::default());
    assert_eq!(simulation.status, "succeeded");
    assert_eq!(simulation.final_state["account.status"], "Allowed");
}

#[test]
fn predicate_expression_operands_drive_runtime_flow() {
    let document = parse_rif_text(
        r#"
        app PredicateExpressions

        things:
          thing Invoice
            field status: State<Draft, Reviewed>
            field subtotal: Int
            field tax: Int

        intent ReviewInvoice

        subject:
          invoice: Invoice

        steps:
          1. Classify computed total
             when: invoice.subtotal + invoice.tax >= 20
             set: invoice.status = Reviewed
             otherwise set: invoice.status = Draft
             changes: invoice.status
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());

    let high_value_state: std::collections::BTreeMap<String, String> = [
        ("invoice.status".to_string(), "Draft".to_string()),
        ("invoice.subtotal".to_string(), "18".to_string()),
        ("invoice.tax".to_string(), "4".to_string()),
    ]
    .into();
    let bytecode = lower_document(&document);
    let high_value = run_bytecode(&bytecode, high_value_state, [].into());
    assert_eq!(high_value.status, "succeeded");
    assert_eq!(high_value.final_state["invoice.status"], "Reviewed");

    let low_value_state: std::collections::BTreeMap<String, String> = [
        ("invoice.status".to_string(), "Reviewed".to_string()),
        ("invoice.subtotal".to_string(), "10".to_string()),
        ("invoice.tax".to_string(), "2".to_string()),
    ]
    .into();
    let low_value = simulate(&document, low_value_state, [].into());
    assert_eq!(low_value.status, "succeeded");
    assert_eq!(low_value.final_state["invoice.status"], "Draft");
}

#[test]
fn predicate_expression_operands_resolve_intent_inputs() {
    let document = parse_rif_text(
        r#"
        app PredicateInputExpressions

        things:
          thing Invoice
            field status: State<Draft, Reviewed>

        intent ReviewInvoice

        subject:
          invoice: Invoice

        inputs:
          subtotal: Int
          tax: Int

        steps:
          1. Classify input total
             when: subtotal + tax >= 20
             set: invoice.status = Reviewed
             otherwise set: invoice.status = Draft
             changes: invoice.status
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());

    let result = run_bytecode(
        &lower_document(&document),
        [
            ("invoice.status".to_string(), "Draft".to_string()),
            ("subtotal".to_string(), "19".to_string()),
            ("tax".to_string(), "4".to_string()),
        ]
        .into(),
        [].into(),
    );
    assert_eq!(result.status, "succeeded");
    assert_eq!(result.final_state["invoice.status"], "Reviewed");
}

#[test]
fn checker_reports_unknown_guard_reference() {
    let document = parse_rif_text(
        r#"
        app BadGuard

        things:
          thing Ticket
            field id: Id<Ticket>

        intent RouteTicket

        subject:
          ticket: Ticket

        steps:
          1. Route
             when: missing.priority is Critical
             set: ticket.id = ticket.id
             changes: ticket.id

        guarantees:
          if this intent succeeds:
            ticket exists
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_UNKNOWN_GUARD_REFERENCE".to_string()));
}

#[test]
fn checker_reports_unknown_references_inside_grouped_predicates() {
    let document = parse_rif_text(
        r#"
        app BadGroupedPredicates

        things:
          thing Invoice
            field status: State<Draft, Finalized>

        intent FinalizeInvoice

        subject:
          invoice: Invoice

        requires:
          not (missing.status is Finalized)

        steps:
          1. Finalize
             when: invoice.status is Draft or missing.tax > 0
             set: invoice.status = Finalized
             changes: invoice.status

        guarantees:
          if this intent succeeds:
            not (missing.total > 0)
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_UNKNOWN_REQUIREMENT_REFERENCE".to_string()));
    assert!(codes.contains(&"EIGL_UNKNOWN_GUARD_REFERENCE".to_string()));
    assert!(codes.contains(&"EIGL_UNKNOWN_GUARANTEE_REFERENCE".to_string()));
}

#[test]
fn checker_reports_unknown_right_hand_predicate_references() {
    let document = parse_rif_text(
        r#"
        app BadRightHandPredicates

        things:
          thing Invoice
            field status: State<Draft, Finalized>
            field expected_status: State<Draft, Finalized>
            field total: Int

        intent FinalizeInvoice

        subject:
          invoice: Invoice

        requires:
          invoice.status is missing.expected_status

        steps:
          1. Finalize
             when: invoice.total > missing.threshold
             set: invoice.status = Finalized
             changes: invoice.status

        guarantees:
          if this intent succeeds:
            invoice.status is missing.final_status
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_UNKNOWN_REQUIREMENT_REFERENCE".to_string()));
    assert!(codes.contains(&"EIGL_UNKNOWN_GUARD_REFERENCE".to_string()));
    assert!(codes.contains(&"EIGL_UNKNOWN_GUARANTEE_REFERENCE".to_string()));
}

#[test]
fn checker_reports_invalid_predicate_operand_types() {
    let document = parse_rif_text(
        r#"
        app BadPredicateTypes

        things:
          thing Invoice
            field status: State<Draft, Finalized>
            field subtotal: Money
            field discount_rate: Decimal
            field starts_at: Time
            field timeout: Duration
            field note: Text

        intent FinalizeInvoice

        subject:
          invoice: Invoice

        requires:
          invoice.status > 0

        steps:
          1. Finalize
             when: invoice.timeout < invoice.starts_at or invoice.note > 0
             set: invoice.status = Finalized
             changes: invoice.status

        guarantees:
          if this intent succeeds:
            invoice.subtotal > invoice.discount_rate
        "#,
    )
    .unwrap();

    let diagnostics = check_document(&document);
    let invalid_predicate_types = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "EIGL_PREDICATE_OPERAND_TYPE")
        .count();
    assert_eq!(invalid_predicate_types, 4, "{diagnostics:#?}");
}

#[test]
fn compute_steps_lower_and_execute_arithmetic_expressions() {
    let document = parse_rif_file(fixture("invoice_app.rif.md")).unwrap();
    assert_eq!(check_document(&document), Vec::new());
    assert_eq!(
        document.intent.steps[0].compute_statements[0].as_str(),
        "invoice.total = invoice.subtotal + invoice.tax"
    );

    let bytecode = lower_document(&document);
    assert!(
        bytecode
            .instructions
            .iter()
            .any(|instruction| instruction.opcode == "COMPUTE"
                && instruction.fields["field"] == "invoice.total"
                && instruction.fields["expression"] == "invoice.subtotal + invoice.tax")
    );
    let program = build_program(&document);
    assert!(
        program
            .permissions
            .iter()
            .any(|permission| permission.kind == "Read" && permission.target == "invoice.subtotal")
    );
    assert!(
        program
            .permissions
            .iter()
            .any(|permission| permission.kind == "Read" && permission.target == "invoice.tax")
    );
    assert!(
        program
            .permissions
            .iter()
            .any(|permission| permission.kind == "Change" && permission.target == "invoice.total")
    );

    let result = run_bytecode(
        &bytecode,
        [
            ("invoice.status".to_string(), "Draft".to_string()),
            ("invoice.subtotal".to_string(), "125".to_string()),
            ("invoice.tax".to_string(), "25".to_string()),
        ]
        .into(),
        [].into(),
    );
    assert_eq!(result.status, "succeeded");
    assert_eq!(result.final_state["invoice.total"], "150");
    assert_eq!(result.final_state["invoice.status"], "Finalized");
    assert!(
        result
            .trace
            .contains(&"COMPUTE invoice.total = invoice.subtotal + invoice.tax".to_string())
    );
}

#[test]
fn local_compute_values_drive_runtime_flow_without_persisting_state() {
    let document = parse_rif_text(
        r#"
        app InvoiceFlow

        things:
          thing Invoice
            field subtotal: Int
            field tax: Int
            field total: Int
            field status: State<Draft, Approved>

        intent CalculateInvoice

        subject:
          invoice: Invoice

        steps:
          1. Calculate local total
             compute: line_total = invoice.subtotal + invoice.tax

          2. Apply approved total
             when: line_total > 0
             set: invoice.total = line_total
             set: invoice.status = Approved

        returns:
          amount: line_total
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());
    let state: std::collections::BTreeMap<String, String> = [
        ("invoice.subtotal".to_string(), "10".to_string()),
        ("invoice.tax".to_string(), "2".to_string()),
        ("invoice.total".to_string(), "0".to_string()),
        ("invoice.status".to_string(), "Draft".to_string()),
    ]
    .into();

    let result = run_bytecode(
        &lower_document(&document),
        state.clone(),
        Default::default(),
    );
    assert_eq!(result.status, "succeeded");
    assert_eq!(result.final_state["invoice.total"], "12");
    assert_eq!(result.final_state["invoice.status"], "Approved");
    assert_eq!(result.outputs["amount"], "12");
    assert!(!result.final_state.contains_key("line_total"));

    let simulation = simulate(&document, state, Default::default());
    assert_eq!(simulation.status, "succeeded");
    assert_eq!(simulation.final_state["invoice.total"], "12");
    assert_eq!(simulation.final_state["invoice.status"], "Approved");
    assert_eq!(simulation.outputs["amount"], "12");
    assert!(!simulation.final_state.contains_key("line_total"));
}

#[test]
fn compute_expressions_respect_precedence_parentheses_and_compact_operators() {
    let document = parse_rif_text(
        r#"
        app Pricing

        things:
          thing Invoice
            field subtotal: Int
            field tax: Int
            field multiplier: Int
            field total: Int
            field adjusted: Int

        intent CalculateInvoice

        subject:
          invoice: Invoice

        steps:
          1. Calculate totals
             compute: invoice.total = invoice.subtotal+invoice.tax*invoice.multiplier
             compute: invoice.adjusted = (invoice.subtotal+invoice.tax)*invoice.multiplier
             changes: invoice.total, invoice.adjusted
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());
    let program = build_program(&document);
    for target in ["invoice.subtotal", "invoice.tax", "invoice.multiplier"] {
        assert!(
            program
                .permissions
                .iter()
                .any(|permission| permission.kind == "Read" && permission.target == target),
            "missing read permission for {target}"
        );
    }

    let bytecode = lower_document(&document);
    let state = [
        ("invoice.subtotal".to_string(), "10".to_string()),
        ("invoice.tax".to_string(), "2".to_string()),
        ("invoice.multiplier".to_string(), "3".to_string()),
    ]
    .into();
    let result = run_bytecode(&bytecode, state, [].into());
    assert_eq!(result.status, "succeeded");
    assert_eq!(result.final_state["invoice.total"], "16");
    assert_eq!(result.final_state["invoice.adjusted"], "36");

    let simulation = simulate(
        &document,
        [
            ("invoice.subtotal".to_string(), "10".to_string()),
            ("invoice.tax".to_string(), "2".to_string()),
            ("invoice.multiplier".to_string(), "3".to_string()),
        ]
        .into(),
        [].into(),
    );
    assert_eq!(simulation.status, "succeeded");
    assert_eq!(simulation.final_state["invoice.total"], "16");
    assert_eq!(simulation.final_state["invoice.adjusted"], "36");
}

#[test]
fn checker_reports_unknown_references_inside_compact_parenthesized_compute_expressions() {
    let document = parse_rif_text(
        r#"
        app BrokenPricing

        things:
          thing Invoice
            field subtotal: Int
            field multiplier: Int
            field total: Int

        intent CalculateInvoice

        subject:
          invoice: Invoice

        steps:
          1. Calculate total
             compute: invoice.total = (invoice.subtotal+missing.tax)*invoice.multiplier
             changes: invoice.total
        "#,
    )
    .unwrap();

    let diagnostics = check_document(&document);
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code
        == "EIGL_UNKNOWN_EXPRESSION_REFERENCE"
        && diagnostic.message.contains("missing.tax")));
}

#[test]
fn compute_expressions_accept_decimal_arithmetic() {
    let document = parse_rif_text(
        r#"
        app Pricing

        things:
          thing Product
            field price: Decimal
            field discount_rate: Decimal
            field final_price: Decimal

        intent PriceProduct

        subject:
          product: Product

        steps:
          1. Apply discount
             compute: product.final_price = product.price-product.price*product.discount_rate
             changes: product.final_price
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());

    let bytecode = lower_document(&document);
    let state = [
        ("product.price".to_string(), "20.50".to_string()),
        ("product.discount_rate".to_string(), "0.10".to_string()),
    ]
    .into();
    let result = run_bytecode(&bytecode, state, [].into());
    assert_eq!(result.status, "succeeded");
    assert_eq!(result.final_state["product.final_price"], "18.45");

    let simulation = simulate(
        &document,
        [
            ("product.price".to_string(), "20.50".to_string()),
            ("product.discount_rate".to_string(), "0.10".to_string()),
        ]
        .into(),
        [].into(),
    );
    assert_eq!(simulation.status, "succeeded");
    assert_eq!(simulation.final_state["product.final_price"], "18.45");
}

#[test]
fn checker_reports_decimal_compute_assigned_to_int_field() {
    let document = parse_rif_text(
        r#"
        app Pricing

        things:
          thing Product
            field price: Decimal
            field quantity: Int

        intent PriceProduct

        subject:
          product: Product

        steps:
          1. Store decimal in int
             compute: product.quantity = product.price + 1
             changes: product.quantity
        "#,
    )
    .unwrap();

    let diagnostics = check_document(&document);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "EIGL_COMPUTE_TYPE")
    );
}

#[test]
fn compute_expressions_accept_money_arithmetic() {
    let document = parse_rif_text(
        r#"
        app Billing

        things:
          thing Invoice
            field subtotal: Money
            field tax: Money
            field discount_rate: Decimal
            field total: Money

        intent CalculateInvoice

        subject:
          invoice: Invoice

        steps:
          1. Calculate total
             compute: invoice.total = invoice.subtotal + invoice.tax - invoice.subtotal * invoice.discount_rate
             changes: invoice.total
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());

    let bytecode = lower_document(&document);
    let state = [
        ("invoice.subtotal".to_string(), "USD:20.00".to_string()),
        ("invoice.tax".to_string(), "USD:1.50".to_string()),
        ("invoice.discount_rate".to_string(), "0.10".to_string()),
    ]
    .into();
    let result = run_bytecode(&bytecode, state, [].into());
    assert_eq!(result.status, "succeeded");
    assert_eq!(result.final_state["invoice.total"], "USD:19.5");

    let simulation = simulate(
        &document,
        [
            ("invoice.subtotal".to_string(), "USD:20.00".to_string()),
            ("invoice.tax".to_string(), "USD:1.50".to_string()),
            ("invoice.discount_rate".to_string(), "0.10".to_string()),
        ]
        .into(),
        [].into(),
    );
    assert_eq!(simulation.status, "succeeded");
    assert_eq!(simulation.final_state["invoice.total"], "USD:19.5");
}

#[test]
fn checker_rejects_invalid_money_arithmetic() {
    let document = parse_rif_text(
        r#"
        app Billing

        things:
          thing Invoice
            field subtotal: Money
            field tax: Money
            field total: Money

        intent CalculateInvoice

        subject:
          invoice: Invoice

        steps:
          1. Multiply money by money
             compute: invoice.total = invoice.subtotal * invoice.tax
             changes: invoice.total
        "#,
    )
    .unwrap();

    let diagnostics = check_document(&document);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "EIGL_COMPUTE_OPERAND_TYPE")
    );
}

#[test]
fn compute_expressions_accept_text_concatenation() {
    let document = parse_rif_text(
        r#"
        app Profiles

        things:
          thing User
            field first_name: Text
            field last_name: Text
            field display_name: Text

        intent FormatUser

        subject:
          user: User

        steps:
          1. Format display name
             compute: user.display_name = user.first_name + " " + user.last_name
             changes: user.display_name
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());

    let bytecode = lower_document(&document);
    let state = [
        ("user.first_name".to_string(), "Ada".to_string()),
        ("user.last_name".to_string(), "Lovelace".to_string()),
    ]
    .into();
    let result = run_bytecode(&bytecode, state, [].into());
    assert_eq!(result.status, "succeeded");
    assert_eq!(result.final_state["user.display_name"], "Ada Lovelace");

    let simulation = simulate(
        &document,
        [
            ("user.first_name".to_string(), "Ada".to_string()),
            ("user.last_name".to_string(), "Lovelace".to_string()),
        ]
        .into(),
        [].into(),
    );
    assert_eq!(simulation.status, "succeeded");
    assert_eq!(simulation.final_state["user.display_name"], "Ada Lovelace");
}

#[test]
fn checker_rejects_invalid_text_arithmetic() {
    let document = parse_rif_text(
        r#"
        app Profiles

        things:
          thing User
            field first_name: Text
            field last_name: Text
            field display_name: Text

        intent FormatUser

        subject:
          user: User

        steps:
          1. Format display name
             compute: user.display_name = user.first_name - user.last_name
             changes: user.display_name
        "#,
    )
    .unwrap();

    let diagnostics = check_document(&document);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "EIGL_COMPUTE_OPERAND_TYPE")
    );
}

#[test]
fn decimal_and_money_predicates_drive_runtime_flow() {
    let document = parse_rif_text(
        r#"
        app BillingPredicates

        things:
          thing Invoice
            field subtotal: Money
            field minimum: Money
            field discount_rate: Decimal
            field status: State<Pending, Captured, Skipped>

        intent CaptureInvoice

        subject:
          invoice: Invoice

        requires:
          invoice.subtotal >= USD:20.00
          invoice.discount_rate < 0.50

        steps:
          1. Classify invoice
             when: invoice.subtotal > invoice.minimum and invoice.discount_rate <= 0.25
             set: invoice.status = Captured
             otherwise set: invoice.status = Skipped
             changes: invoice.status

        guarantees:
          if this intent succeeds:
            invoice.subtotal >= invoice.minimum
            invoice.discount_rate <= 0.25
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());

    let state: std::collections::BTreeMap<String, String> = [
        ("invoice.subtotal".to_string(), "USD:20.00".to_string()),
        ("invoice.minimum".to_string(), "USD:10.00".to_string()),
        ("invoice.discount_rate".to_string(), "0.25".to_string()),
        ("invoice.status".to_string(), "Pending".to_string()),
    ]
    .into();

    let bytecode = lower_document(&document);
    let result = run_bytecode(&bytecode, state.clone(), [].into());
    assert_eq!(result.status, "succeeded");
    assert_eq!(result.final_state["invoice.status"], "Captured");
    assert!(
        result
            .trace
            .contains(&"ASSERT invoice.subtotal >= invoice.minimum".to_string())
    );

    let simulation = simulate(&document, state, [].into());
    assert_eq!(simulation.status, "succeeded");
    assert_eq!(simulation.final_state["invoice.status"], "Captured");
}

#[test]
fn time_and_duration_predicates_drive_runtime_flow() {
    let document = parse_rif_text(
        r#"
        app SchedulingPredicates

        things:
          thing Job
            field starts_at: Time
            field deadline: Time
            field timeout: Duration
            field status: State<Pending, Scheduled, Expired>

        intent ScheduleJob

        subject:
          job: Job

        requires:
          job.starts_at < job.deadline
          job.timeout <= PT1H

        steps:
          1. Classify job
             when: job.deadline >= 2026-05-20T10:00:00Z and job.timeout == PT30M
             set: job.status = Scheduled
             otherwise set: job.status = Expired
             changes: job.status

        guarantees:
          if this intent succeeds:
            job.starts_at < job.deadline
            job.timeout == PT30M
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());

    let state: std::collections::BTreeMap<String, String> = [
        (
            "job.starts_at".to_string(),
            "2026-05-20T09:30:00Z".to_string(),
        ),
        (
            "job.deadline".to_string(),
            "2026-05-20T10:00:00Z".to_string(),
        ),
        ("job.timeout".to_string(), "PT30M".to_string()),
        ("job.status".to_string(), "Pending".to_string()),
    ]
    .into();

    let bytecode = lower_document(&document);
    let result = run_bytecode(&bytecode, state.clone(), [].into());
    assert_eq!(result.status, "succeeded");
    assert_eq!(result.final_state["job.status"], "Scheduled");
    assert!(
        result
            .trace
            .contains(&"ASSERT job.starts_at < job.deadline".to_string())
    );

    let simulation = simulate(&document, state, [].into());
    assert_eq!(simulation.status, "succeeded");
    assert_eq!(simulation.final_state["job.status"], "Scheduled");
}

#[test]
fn repeat_steps_lower_and_execute_until_state_changes() {
    let document = parse_rif_text(
        r#"
        app QueueDrain

        things:
          thing Queue
            field count: Int

        intent DrainQueue

        subject:
          queue: Queue

        steps:
          1. Drain one item
             repeat while: queue.count > 0
             compute: queue.count = queue.count - 1
             changes: queue.count

        guarantees:
          if this intent succeeds:
            queue.count is 0
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());
    assert_eq!(
        document.intent.steps[0].repeat_while.as_deref(),
        Some("queue.count > 0")
    );

    let bytecode = lower_document(&document);
    assert!(
        bytecode
            .instructions
            .iter()
            .any(|instruction| instruction.opcode == "END_STEP"
                && instruction.fields["step"] == "Drain one item"
                && instruction.fields["repeat_while"] == "queue.count > 0")
    );

    let result = run_bytecode(
        &bytecode,
        [("queue.count".to_string(), "3".to_string())].into(),
        [].into(),
    );
    assert_eq!(result.status, "succeeded");
    assert_eq!(result.final_state["queue.count"], "0");
    assert!(
        result
            .trace
            .iter()
            .filter(|entry| *entry == "BEGIN Drain one item")
            .count()
            >= 3
    );

    let simulation = simulate(
        &document,
        [("queue.count".to_string(), "2".to_string())].into(),
        [].into(),
    );
    assert_eq!(simulation.status, "succeeded");
    assert_eq!(simulation.final_state["queue.count"], "0");
}

#[test]
fn otherwise_branches_lower_and_execute_alternative_effects() {
    let document = parse_rif_text(
        r#"
        app BranchInvoice

        things:
          thing Invoice
            field total: Int
            field status: State<Paid, Free>

        intent ClassifyInvoice

        subject:
          invoice: Invoice

        steps:
          1. Classify invoice
             when: invoice.total > 0
             set: invoice.status = Paid
             otherwise set: invoice.status = Free

        guarantees:
          if this intent succeeds:
            invoice.status is Paid or invoice.status is Free
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());
    assert_eq!(
        document.intent.steps[0].otherwise_set_statements[0].as_str(),
        "invoice.status = Free"
    );

    let bytecode = lower_document(&document);
    assert!(
        bytecode
            .instructions
            .iter()
            .any(|instruction| instruction.opcode == "ELSE_BRANCH"
                && instruction.fields["step"] == "Classify invoice")
    );

    let paid = run_bytecode(
        &bytecode,
        [
            ("invoice.total".to_string(), "10".to_string()),
            ("invoice.status".to_string(), "Draft".to_string()),
        ]
        .into(),
        [].into(),
    );
    assert_eq!(paid.status, "succeeded");
    assert_eq!(paid.final_state["invoice.status"], "Paid");

    let free = run_bytecode(
        &bytecode,
        [
            ("invoice.total".to_string(), "0".to_string()),
            ("invoice.status".to_string(), "Draft".to_string()),
        ]
        .into(),
        [].into(),
    );
    assert_eq!(free.status, "succeeded");
    assert_eq!(free.final_state["invoice.status"], "Free");

    let simulated = simulate(
        &document,
        [
            ("invoice.total".to_string(), "0".to_string()),
            ("invoice.status".to_string(), "Draft".to_string()),
        ]
        .into(),
        [].into(),
    );
    assert_eq!(simulated.status, "succeeded");
    assert_eq!(simulated.final_state["invoice.status"], "Free");
}

#[test]
fn invoke_steps_lower_and_execute_nested_workflows() {
    let document = parse_rif_file(fixture("invoice_workflow_app.rif.md")).unwrap();
    assert_eq!(check_document(&document), Vec::new());
    assert_eq!(
        document.intent.steps[0]
            .invoke
            .as_ref()
            .map(|invoke| invoke.target.as_str()),
        Some("MarkPaid")
    );
    assert_eq!(
        document.intent.steps[0]
            .otherwise_invoke
            .as_ref()
            .map(|invoke| invoke.target.as_str()),
        Some("MarkFree")
    );

    let bytecode = lower_document(&document);
    assert!(
        bytecode
            .instructions
            .iter()
            .any(|instruction| instruction.opcode == "CALL_INTENT"
                && instruction.fields["target"] == "MarkPaid")
    );

    let paid = run_bytecode(
        &bytecode,
        [
            ("invoice.total".to_string(), "10".to_string()),
            ("invoice.status".to_string(), "Draft".to_string()),
        ]
        .into(),
        [].into(),
    );
    assert_eq!(paid.status, "succeeded");
    assert_eq!(paid.final_state["invoice.status"], "Paid");

    let free = simulate(
        &document,
        [
            ("invoice.total".to_string(), "0".to_string()),
            ("invoice.status".to_string(), "Draft".to_string()),
        ]
        .into(),
        [].into(),
    );
    assert_eq!(free.status, "succeeded");
    assert_eq!(free.final_state["invoice.status"], "Free");
}

#[test]
fn invoke_steps_remap_bound_subject_fields() {
    let document = parse_rif_text(
        r#"
        app InvoiceWorkflow

        things:
          thing Invoice
            field status: State<Draft, Paid>

        intent PayInvoice

        subject:
          invoice: Invoice

        steps:
          1. Reuse bill workflow
             invoke: MarkBillPaid(bill = invoice)

        intent MarkBillPaid

        subject:
          bill: Invoice

        steps:
          1. Mark bill paid
             set: bill.status = Paid
        "#,
    )
    .unwrap();
    assert_eq!(check_document(&document), Vec::new());

    let bytecode = lower_document(&document);
    let bytecode_result = run_bytecode(
        &bytecode,
        [("invoice.status".to_string(), "Draft".to_string())].into(),
        [].into(),
    );
    assert_eq!(bytecode_result.status, "succeeded");
    assert_eq!(bytecode_result.final_state["invoice.status"], "Paid");

    let simulated = simulate(
        &document,
        [("invoice.status".to_string(), "Draft".to_string())].into(),
        [].into(),
    );
    assert_eq!(simulated.status, "succeeded");
    assert_eq!(simulated.final_state["invoice.status"], "Paid");
}

#[test]
fn invoke_steps_accept_expression_bindings_for_child_inputs() {
    let document = parse_rif_text(
        r#"
        app InvoiceWorkflow

        things:
          thing Invoice
            field subtotal: Int
            field tax: Int
            field total: Int

        intent FinalizeInvoice

        subject:
          invoice: Invoice

        steps:
          1. Delegate total calculation
             invoke: StoreTotal(total = invoice.subtotal + invoice.tax)

        intent StoreTotal

        subject:
          invoice: Invoice

        inputs:
          total: Int

        steps:
          1. Store calculated total
             set: invoice.total = total
        "#,
    )
    .unwrap();

    assert_eq!(check_document(&document), Vec::new());

    let result = run_bytecode(
        &lower_document(&document),
        [
            ("invoice.subtotal".to_string(), "19".to_string()),
            ("invoice.tax".to_string(), "4".to_string()),
        ]
        .into(),
        [].into(),
    );
    assert_eq!(result.status, "succeeded");
    assert_eq!(result.final_state["invoice.total"], "23");
}

#[test]
fn checker_rejects_local_compute_values_before_invoke_bindings() {
    let document = parse_rif_text(
        r#"
        app FutureInvokeBindingLocal

        things:
          thing Invoice
            field subtotal: Int
            field tax: Int
            field total: Int

        intent FinalizeInvoice

        subject:
          invoice: Invoice

        steps:
          1. Delegate total too early
             invoke: StoreTotal(total = line_total)

          2. Calculate local total
             compute: line_total = invoice.subtotal + invoice.tax

        intent StoreTotal

        subject:
          invoice: Invoice

        inputs:
          total: Int

        steps:
          1. Store calculated total
             set: invoice.total = total
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_LOCAL_VALUE_UNAVAILABLE".to_string()));
}

#[test]
fn checker_requires_missing_invocation_bindings() {
    let document = parse_rif_text(
        r#"
        app BrokenInvoiceWorkflow

        things:
          thing Invoice
            field status: State<Draft, Paid>

        intent PayInvoice

        subject:
          invoice: Invoice

        steps:
          1. Reuse bill workflow without binding its subject
             invoke: MarkBillPaid

        intent MarkBillPaid

        subject:
          bill: Invoice

        steps:
          1. Mark bill paid
             set: bill.status = Paid
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_MISSING_INVOKE_BINDING".to_string()));
}

#[test]
fn parallel_invokes_lower_and_merge_independent_workflows() {
    let document = parse_rif_file(fixture("parallel_invoice_app.rif.md")).unwrap();
    assert_eq!(check_document(&document), Vec::new());
    assert_eq!(
        document.intent.steps[0]
            .parallel_invokes
            .iter()
            .map(|invoke| invoke.target.as_str())
            .collect::<Vec<_>>(),
        vec!["CapturePayment", "ReserveShipment"]
    );
    assert_eq!(
        document.intent.steps[0]
            .otherwise_parallel_invokes
            .iter()
            .map(|invoke| invoke.target.as_str())
            .collect::<Vec<_>>(),
        vec!["MarkFree", "ArchiveInvoice"]
    );

    let bytecode = lower_document(&document);
    assert!(
        bytecode
            .instructions
            .iter()
            .any(|instruction| instruction.opcode == "CALL_INTENT_PARALLEL"
                && instruction.fields["targets"] == "CapturePayment,ReserveShipment")
    );

    let paid = run_bytecode(
        &bytecode,
        [
            ("invoice.total".to_string(), "10".to_string()),
            ("invoice.status".to_string(), "Draft".to_string()),
            ("invoice.payment_status".to_string(), "Pending".to_string()),
            ("invoice.shipping_status".to_string(), "Pending".to_string()),
            ("invoice.archival_status".to_string(), "Active".to_string()),
        ]
        .into(),
        [].into(),
    );
    assert_eq!(paid.status, "succeeded");
    assert_eq!(paid.final_state["invoice.payment_status"], "Captured");
    assert_eq!(paid.final_state["invoice.shipping_status"], "Queued");

    let free = simulate(
        &document,
        [
            ("invoice.total".to_string(), "0".to_string()),
            ("invoice.status".to_string(), "Draft".to_string()),
            ("invoice.payment_status".to_string(), "Pending".to_string()),
            ("invoice.shipping_status".to_string(), "Pending".to_string()),
            ("invoice.archival_status".to_string(), "Active".to_string()),
        ]
        .into(),
        [].into(),
    );
    assert_eq!(free.status, "succeeded");
    assert_eq!(free.final_state["invoice.status"], "Free");
    assert_eq!(free.final_state["invoice.archival_status"], "Archived");

    let conflicting = parse_rif_text(
        r#"
        app ParallelConflict

        things:
          thing Invoice
            field status: State<Draft, Paid, Cancelled>

        intent ProcessInvoice

        subject:
          invoice: Invoice

        steps:
          1. Split work
             parallel invoke: MarkPaid, MarkCancelled

        intent MarkPaid

        subject:
          invoice: Invoice

        steps:
          1. Mark invoice paid
             set: invoice.status = Paid

        intent MarkCancelled

        subject:
          invoice: Invoice

        steps:
          1. Mark invoice cancelled
             set: invoice.status = Cancelled
        "#,
    )
    .unwrap();
    let result = run_bytecode(
        &lower_document(&conflicting),
        [("invoice.status".to_string(), "Draft".to_string())].into(),
        [].into(),
    );
    assert_eq!(result.status, "failed");
    assert_eq!(result.failure.as_deref(), Some("ParallelJoinConflict"));
}

#[test]
fn intent_returns_publish_explicit_result_aliases() {
    let document = parse_rif_file(fixture("order_review_app.rif.md")).unwrap();
    assert_eq!(check_document(&document), Vec::new());
    assert_eq!(
        document.intent.steps[0].invoke.as_ref().unwrap().target,
        "ApproveOrder"
    );
    assert_eq!(
        document.intent.steps[0]
            .invoke
            .as_ref()
            .unwrap()
            .bindings
            .get("should_approve"),
        Some(&"approved".to_string())
    );
    assert_eq!(
        document.intent.returns,
        vec![
            eigl::rif_model::ReturnValue {
                name: "decision".to_string(),
                source: "order.status".to_string(),
            },
            eigl::rif_model::ReturnValue {
                name: "note".to_string(),
                source: "order.review_note".to_string(),
            },
        ]
    );

    let bytecode = lower_document(&document);
    let approved = run_bytecode(
        &bytecode,
        [
            ("order.status".to_string(), "Draft".to_string()),
            ("order.review_note".to_string(), "None".to_string()),
            ("approved".to_string(), "true".to_string()),
        ]
        .into(),
        [].into(),
    );
    assert_eq!(approved.status, "succeeded");
    assert_eq!(approved.outputs["decision"], "Approved");
    assert_eq!(approved.outputs["note"], "ApprovedByPolicy");

    let rejected = simulate(
        &document,
        [
            ("order.status".to_string(), "Draft".to_string()),
            ("order.review_note".to_string(), "None".to_string()),
            ("approved".to_string(), "false".to_string()),
        ]
        .into(),
        [].into(),
    );
    assert_eq!(rejected.status, "succeeded");
    assert_eq!(rejected.outputs["decision"], "Rejected");
    assert_eq!(rejected.outputs["note"], "RejectedByPolicy");
}

#[test]
fn intent_returns_publish_typed_object_aliases() {
    let document = parse_rif_text(
        r#"
        app ObjectReturns

        things:
          thing Ticket
            field id: Text
            field title: Text
            field estimate: Int

        intent ShowTicket

        subject:
          ticket: Ticket

        returns:
          result: ticket
        "#,
    )
    .unwrap();
    assert_eq!(check_document(&document), Vec::new());

    let state: std::collections::BTreeMap<String, String> = [
        ("ticket.id".to_string(), "T-1".to_string()),
        ("ticket.title".to_string(), "Printer".to_string()),
        ("ticket.estimate".to_string(), "3".to_string()),
    ]
    .into();
    let expected = r#"{"estimate":3,"id":"T-1","title":"Printer"}"#;

    let bytecode = run_bytecode(&lower_document(&document), state.clone(), [].into());
    assert_eq!(bytecode.status, "succeeded");
    assert_eq!(bytecode.outputs["result"], expected);

    let simulated = simulate(&document, state, [].into());
    assert_eq!(simulated.status, "succeeded");
    assert_eq!(simulated.outputs["result"], expected);
}

#[test]
fn checker_rejects_duplicate_return_aliases() {
    let document = parse_rif_text(
        r#"
        app DuplicateReturns

        things:
          thing Ticket
            field id: Text
            field title: Text

        intent ShowTicket

        subject:
          ticket: Ticket

        returns:
          result: ticket.id
          result: ticket.title
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_DUPLICATE_RETURN".to_string()));
}

#[test]
fn intent_returns_publish_literal_aliases() {
    let document = parse_rif_text(
        r#"
        app LiteralReturns

        things:
          thing Invoice
            field status: State<Draft, Paid>

        intent PayInvoice

        subject:
          invoice: Invoice

        steps:
          1. Mark invoice paid
             set: invoice.status = Paid

        returns:
          status: invoice.status
          message: "paid"
          count: 3
        "#,
    )
    .unwrap();
    assert_eq!(check_document(&document), Vec::new());

    let bytecode = run_bytecode(
        &lower_document(&document),
        [("invoice.status".to_string(), "Draft".to_string())].into(),
        [].into(),
    );
    assert_eq!(bytecode.status, "succeeded");
    assert_eq!(bytecode.outputs["status"], "Paid");
    assert_eq!(bytecode.outputs["message"], "paid");
    assert_eq!(bytecode.outputs["count"], "3");

    let simulated = simulate(
        &document,
        [("invoice.status".to_string(), "Draft".to_string())].into(),
        [].into(),
    );
    assert_eq!(simulated.status, "succeeded");
    assert_eq!(simulated.outputs["status"], "Paid");
    assert_eq!(simulated.outputs["message"], "paid");
    assert_eq!(simulated.outputs["count"], "3");
}

#[test]
fn intent_returns_publish_expression_aliases() {
    let document = parse_rif_text(
        r#"
        app ExpressionReturns

        things:
          thing Invoice
            field subtotal: Int
            field tax: Int

        intent SummarizeInvoice

        subject:
          invoice: Invoice

        returns:
          total: invoice.subtotal + invoice.tax
        "#,
    )
    .unwrap();
    assert_eq!(check_document(&document), Vec::new());

    let bytecode = run_bytecode(
        &lower_document(&document),
        [
            ("invoice.subtotal".to_string(), "19".to_string()),
            ("invoice.tax".to_string(), "4".to_string()),
        ]
        .into(),
        [].into(),
    );
    assert_eq!(bytecode.status, "succeeded");
    assert_eq!(bytecode.outputs["total"], "23");

    let simulated = simulate(
        &document,
        [
            ("invoice.subtotal".to_string(), "19".to_string()),
            ("invoice.tax".to_string(), "4".to_string()),
        ]
        .into(),
        [].into(),
    );
    assert_eq!(simulated.status, "succeeded");
    assert_eq!(simulated.outputs["total"], "23");
}

#[test]
fn checker_reports_unknown_return_field_references() {
    let document = parse_rif_text(
        r#"
        app BrokenReturns

        things:
          thing Invoice
            field status: State<Draft, Paid>

        intent PayInvoice

        subject:
          invoice: Invoice

        steps:
          1. Mark invoice paid
             set: invoice.status = Paid

        returns:
          missing: invoice.missing
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_UNKNOWN_RETURN_REFERENCE".to_string()));
}

#[test]
fn checker_reports_unknown_compute_expression_reference() {
    let document = parse_rif_text(
        r#"
        app BrokenCompute

        things:
          thing Invoice
            field subtotal: Int
            field total: Int

        intent CalculateInvoice

        subject:
          invoice: Invoice

        steps:
          1. Calculate invoice total
             compute: invoice.total = invoice.subtotal + missing.tax
             changes: invoice.total

        guarantees:
          if this intent succeeds:
            invoice.total > 0
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_UNKNOWN_EXPRESSION_REFERENCE".to_string()));
}

#[test]
fn checker_rejects_step_outputs_before_they_are_available() {
    let document = parse_rif_text(
        r#"
        app FutureOutputReference

        things:
          thing Ticket
            field id: Text
            field title: Text

        operations:
          operation Catalog.title(id: Text) -> Text

        intent FillTicket

        subject:
          ticket: Ticket

        steps:
          1. Store title too early
             set: ticket.title = fetched_title

          2. Fetch title
             call: Catalog.title(ticket.id)
             output: fetched_title: Text
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_OUTPUT_UNAVAILABLE".to_string()));
}

#[test]
fn checker_rejects_step_outputs_before_invoke_bindings() {
    let document = parse_rif_text(
        r#"
        app FutureInvokeBindingOutput

        things:
          thing Ticket
            field id: Text
            field title: Text

        operations:
          operation Catalog.title(id: Text) -> Text

        intent FillTicket

        subject:
          ticket: Ticket

        steps:
          1. Delegate title too early
             invoke: StoreTitle(title = fetched_title)

          2. Fetch title
             call: Catalog.title(ticket.id)
             output: fetched_title: Text

        intent StoreTitle

        subject:
          ticket: Ticket

        inputs:
          title: Text

        steps:
          1. Store title
             set: ticket.title = title
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_OUTPUT_UNAVAILABLE".to_string()));
}

#[test]
fn checker_rejects_duplicate_step_output_names() {
    let document = parse_rif_text(
        r#"
        app DuplicateStepOutputs

        things:
          thing Ticket
            field id: Text

        operations:
          operation Catalog.title(id: Text) -> Text
          operation Catalog.summary(id: Text) -> Text

        intent FillTicket

        subject:
          ticket: Ticket

        steps:
          1. Fetch title
             call: Catalog.title(ticket.id)
             output: fetched_text: Text

          2. Fetch summary
             call: Catalog.summary(ticket.id)
             output: fetched_text: Text
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_DUPLICATE_OUTPUT".to_string()));
}

#[test]
fn checker_rejects_local_compute_values_before_they_are_available() {
    let document = parse_rif_text(
        r#"
        app FutureLocalComputeReference

        things:
          thing Invoice
            field subtotal: Int
            field tax: Int
            field total: Int

        intent CalculateInvoice

        subject:
          invoice: Invoice

        steps:
          1. Store total too early
             set: invoice.total = line_total

          2. Calculate local total
             compute: line_total = invoice.subtotal + invoice.tax
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_LOCAL_VALUE_UNAVAILABLE".to_string()));
}

#[test]
fn checker_rejects_local_compute_values_in_intent_requirements() {
    let document = parse_rif_text(
        r#"
        app RequirementLocalComputeReference

        things:
          thing Invoice
            field subtotal: Int
            field tax: Int

        intent CalculateInvoice

        subject:
          invoice: Invoice

        requires:
          line_total > 0

        steps:
          1. Calculate local total
             compute: line_total = invoice.subtotal + invoice.tax
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_LOCAL_VALUE_UNAVAILABLE".to_string()));
}

#[test]
fn checker_rejects_outputs_from_skippable_guarded_steps() {
    let document = parse_rif_text(
        r#"
        app GuardedOutputReference

        things:
          thing Ticket
            field id: Text
            field title: Text

        operations:
          operation Catalog.title(id: Text) -> Text

        intent FillTicket

        subject:
          ticket: Ticket

        steps:
          1. Fetch title conditionally
             when: ticket.id == "T-1"
             call: Catalog.title(ticket.id)
             output: fetched_title: Text

          2. Store title
             set: ticket.title = fetched_title
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_OUTPUT_UNAVAILABLE".to_string()));
}

#[test]
fn checker_rejects_local_computes_from_skippable_guarded_steps() {
    let document = parse_rif_text(
        r#"
        app GuardedLocalComputeReference

        things:
          thing Invoice
            field subtotal: Int
            field tax: Int
            field total: Int

        intent CalculateInvoice

        subject:
          invoice: Invoice

        steps:
          1. Calculate local total conditionally
             when: invoice.subtotal > 0
             compute: line_total = invoice.subtotal + invoice.tax

          2. Store total
             set: invoice.total = line_total
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_LOCAL_VALUE_UNAVAILABLE".to_string()));
}

#[test]
fn checker_rejects_unavailable_step_outputs_in_returns() {
    let document = parse_rif_text(
        r#"
        app GuardedOutputReturn

        things:
          thing Ticket
            field id: Text

        operations:
          operation Catalog.title(id: Text) -> Text

        intent FetchTicketTitle

        subject:
          ticket: Ticket

        steps:
          1. Fetch title conditionally
             when: ticket.id == "T-1"
             call: Catalog.title(ticket.id)
             output: fetched_title: Text

        returns:
          title: fetched_title
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_OUTPUT_UNAVAILABLE".to_string()));
}

#[test]
fn checker_rejects_unavailable_local_computes_in_returns() {
    let document = parse_rif_text(
        r#"
        app GuardedLocalComputeReturn

        things:
          thing Invoice
            field subtotal: Int
            field tax: Int

        intent CalculateInvoice

        subject:
          invoice: Invoice

        steps:
          1. Calculate total conditionally
             when: invoice.subtotal > 0
             compute: line_total = invoice.subtotal + invoice.tax

        returns:
          amount: line_total
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_LOCAL_VALUE_UNAVAILABLE".to_string()));
}

#[test]
fn checker_rejects_unavailable_step_outputs_in_guarantees() {
    let document = parse_rif_text(
        r#"
        app GuardedOutputGuarantee

        things:
          thing Ticket
            field id: Text

        operations:
          operation Catalog.title(id: Text) -> Text

        intent FetchTicketTitle

        subject:
          ticket: Ticket

        steps:
          1. Fetch title conditionally
             when: ticket.id == "T-1"
             call: Catalog.title(ticket.id)
             output: fetched_title: Text

        guarantees:
          if this intent succeeds:
            fetched_title exists
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_OUTPUT_UNAVAILABLE".to_string()));
}

#[test]
fn checker_rejects_unavailable_local_computes_in_guarantees() {
    let document = parse_rif_text(
        r#"
        app GuardedLocalComputeGuarantee

        things:
          thing Invoice
            field subtotal: Int
            field tax: Int

        intent CalculateInvoice

        subject:
          invoice: Invoice

        steps:
          1. Calculate total conditionally
             when: invoice.subtotal > 0
             compute: line_total = invoice.subtotal + invoice.tax

        guarantees:
          if this intent succeeds:
            line_total > 0
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_LOCAL_VALUE_UNAVAILABLE".to_string()));
}

#[test]
fn checker_validates_compute_expression_types() {
    let document = parse_rif_text(
        r#"
        app BrokenComputeTypes

        things:
          thing Invoice
            field subtotal: Int
            field tax: Int
            field total: Int
            field status: State<Draft, Finalized>
            field note: Text

        intent CalculateInvoice

        subject:
          invoice: Invoice

        steps:
          1. Add state as number
             reads: invoice.status, invoice.tax
             compute: invoice.total = invoice.status + invoice.tax
             changes: invoice.total

          2. Store number in text
             reads: invoice.subtotal, invoice.tax
             compute: invoice.note = invoice.subtotal + invoice.tax
             changes: invoice.note

        guarantees:
          if this intent succeeds:
            invoice exists
        "#,
    )
    .unwrap();

    let codes: Vec<_> = check_document(&document)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"EIGL_COMPUTE_OPERAND_TYPE".to_string()));
    assert!(codes.contains(&"EIGL_COMPUTE_TYPE".to_string()));
}

#[test]
fn bytecode_vm_enforces_guarantee_assertions() {
    let document = parse_rif_file(fixture("invoice_app.rif.md")).unwrap();
    let bytecode = lower_document(&document);

    let success = run_bytecode(
        &bytecode,
        [
            ("invoice.status".to_string(), "Draft".to_string()),
            ("invoice.subtotal".to_string(), "10".to_string()),
            ("invoice.tax".to_string(), "1".to_string()),
        ]
        .into(),
        [].into(),
    );
    assert_eq!(success.status, "succeeded");
    assert!(
        success
            .trace
            .contains(&"ASSERT invoice.total > 0".to_string())
    );

    let violation_document = parse_rif_file(fixture("bad_contract.rif.md")).unwrap();
    let violation_bytecode = lower_document(&violation_document);
    let violation = run_bytecode(&violation_bytecode, [].into(), [].into());
    assert_eq!(violation.status, "failed");
    assert_eq!(violation.failure.as_deref(), Some("GuaranteeFailed"));
    assert!(
        violation
            .trace
            .contains(&"ASSERT FAILED invoice.total > 0".to_string())
    );
}

#[test]
fn bytecode_vm_enforces_requirements_without_defaulting_state() {
    let document = parse_rif_file(fixture("invoice_app.rif.md")).unwrap();
    let bytecode = lower_document(&document);

    let missing_tax = run_bytecode(
        &bytecode,
        [
            ("invoice.status".to_string(), "Draft".to_string()),
            ("invoice.subtotal".to_string(), "10".to_string()),
        ]
        .into(),
        [].into(),
    );
    assert_eq!(missing_tax.status, "failed");
    assert_eq!(missing_tax.failure.as_deref(), Some("RequirementFailed"));
    assert!(
        missing_tax
            .trace
            .contains(&"CHECK FAILED invoice.tax > 0".to_string())
    );
    assert!(
        !missing_tax
            .trace
            .contains(&"COMPUTE invoice.total = invoice.subtotal + invoice.tax".to_string())
    );

    let wrong_status = run_bytecode(
        &bytecode,
        [
            ("invoice.status".to_string(), "Submitted".to_string()),
            ("invoice.subtotal".to_string(), "10".to_string()),
            ("invoice.tax".to_string(), "1".to_string()),
        ]
        .into(),
        [].into(),
    );
    assert_eq!(wrong_status.status, "failed");
    assert_eq!(wrong_status.failure.as_deref(), Some("RequirementFailed"));
    assert!(
        wrong_status
            .trace
            .contains(&"CHECK FAILED invoice.status is Draft".to_string())
    );
}

#[test]
fn bytecode_run_returns_failure_when_guarantee_fails() {
    let document = parse_rif_text(
        r#"
        app BrokenRuntimeContract

        things:
          thing Invoice
            field total: Int

        intent BreakContract

        subject:
          invoice: Invoice

        steps:
          1. Set invalid total
             set: invoice.total = 0
             changes: invoice.total

        guarantees:
          if this intent succeeds:
            invoice.total > 0
        "#,
    )
    .unwrap();
    let bytecode = lower_document(&document);
    let result = run_bytecode(&bytecode, [].into(), [].into());

    assert_eq!(result.status, "failed");
    assert_eq!(result.failure.as_deref(), Some("GuaranteeFailed"));
}

#[test]
fn cli_acceptance_commands_are_runnable() {
    let binary = env!("CARGO_BIN_EXE_eigl");

    let check_valid = Command::new(binary)
        .args(["check", "examples/confirm_order.rif.md"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(check_valid.status.success());
    assert!(String::from_utf8_lossy(&check_valid.stdout).contains("no diagnostics"));

    let check_invalid = Command::new(binary)
        .args(["check", "examples/bad_parallel_update.rif.md"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(!check_invalid.status.success());
    assert!(String::from_utf8_lossy(&check_invalid.stdout).contains("EIGL_PERMISSION_CONFLICT"));

    let graph = Command::new(binary)
        .args(["graph", "examples/confirm_order.rif.md"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(graph.status.success());
    let graph_stdout = String::from_utf8_lossy(&graph.stdout);
    assert!(graph_stdout.contains("\"modules\""));
    assert!(graph_stdout.contains("\"nodes\""));

    let views = Command::new(binary)
        .args(["views", "examples/confirm_order.rif.md"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(views.status.success());
    let views_stdout = String::from_utf8_lossy(&views.stdout);
    assert!(views_stdout.contains("Flow"));
    assert!(views_stdout.contains("Permissions"));
    assert!(views_stdout.contains("Explanation"));

    let simulate = Command::new(binary)
        .args([
            "simulate",
            "examples/confirm_order.rif.md",
            "order.status=Draft",
            "order.items.count=2",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(simulate.status.success());
    let simulate_stdout = String::from_utf8_lossy(&simulate.stdout);
    assert!(simulate_stdout.contains("succeeded"));
    assert!(simulate_stdout.contains("order.status=Confirmed"));

    let rsl_check = Command::new(binary)
        .args(["check", "examples/confirm_order.rsl.md"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(rsl_check.status.success());
    assert!(String::from_utf8_lossy(&rsl_check.stdout).contains("no diagnostics"));

    let rsl_run = Command::new(binary)
        .args([
            "run",
            "examples/confirm_order.rsl.md",
            "order.status=Draft",
            "order.items.count=2",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(rsl_run.status.success());
    let rsl_run_stdout = String::from_utf8_lossy(&rsl_run.stdout);
    assert!(rsl_run_stdout.contains("bytecode succeeded"));
    assert!(rsl_run_stdout.contains("order.status=Confirmed"));

    let sentence_check = Command::new(binary)
        .args(["check", "examples/domain_sentence_app.rsl.md"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(sentence_check.status.success());
    assert!(String::from_utf8_lossy(&sentence_check.stdout).contains("no diagnostics"));

    let lower = Command::new(binary)
        .args(["lower", "examples/confirm_order.rif.md"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(lower.status.success());
    let lower_stdout = String::from_utf8_lossy(&lower.stdout);
    assert!(lower_stdout.contains("\"opcode\":\"CALL_EFFECT\""));

    let run = Command::new(binary)
        .args([
            "run",
            "examples/confirm_order.rif.md",
            "order.status=Draft",
            "order.items.count=2",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(run.status.success());
    let run_stdout = String::from_utf8_lossy(&run.stdout);
    assert!(run_stdout.contains("bytecode succeeded"));
    assert!(run_stdout.contains("order.status=Confirmed"));

    let run_invoice = Command::new(binary)
        .args([
            "run",
            "examples/invoice_app.rif.md",
            "invoice.status=Draft",
            "invoice.subtotal=1",
            "invoice.tax=1",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(run_invoice.status.success());
    let run_invoice_stdout = String::from_utf8_lossy(&run_invoice.stdout);
    assert!(run_invoice_stdout.contains("invoice.total=2"));
    assert!(run_invoice_stdout.contains("invoice.status=Finalized"));

    let run_bad_contract = Command::new(binary)
        .args(["run", "examples/bad_contract.rif.md"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(!run_bad_contract.status.success());
    let bad_contract_stdout = String::from_utf8_lossy(&run_bad_contract.stdout);
    assert!(bad_contract_stdout.contains("bytecode failed"));
    assert!(bad_contract_stdout.contains("failure=GuaranteeFailed"));
    assert!(bad_contract_stdout.contains("ASSERT FAILED invoice.total > 0"));
}

#[test]
fn cli_run_accepts_runtime_state_assignments() {
    let binary = env!("CARGO_BIN_EXE_eigl");

    let run_invoice = Command::new(binary)
        .args([
            "run",
            "examples/invoice_app.rif.md",
            "invoice.status=Draft",
            "invoice.subtotal=40",
            "invoice.tax=2",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(run_invoice.status.success());
    let stdout = String::from_utf8_lossy(&run_invoice.stdout);
    assert!(stdout.contains("bytecode succeeded"));
    assert!(stdout.contains("invoice.subtotal=40"));
    assert!(stdout.contains("invoice.tax=2"));
    assert!(stdout.contains("invoice.total=42"));
}

#[test]
fn cli_run_loads_and_saves_state_files() {
    let binary = env!("CARGO_BIN_EXE_eigl");
    let state_in = temp_path("in");
    let state_out = temp_path("out");
    fs::write(&state_in, "invoice.status=Draft\ninvoice.subtotal=40\n").unwrap();

    let run_invoice = Command::new(binary)
        .args([
            "run",
            "examples/invoice_app.rif.md",
            "--state-in",
            state_in.to_str().unwrap(),
            "--state-out",
            state_out.to_str().unwrap(),
            "invoice.tax=2",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(run_invoice.status.success());
    let stdout = String::from_utf8_lossy(&run_invoice.stdout);
    assert!(stdout.contains("bytecode succeeded"));
    assert!(stdout.contains("invoice.status=Finalized"));
    assert!(stdout.contains("invoice.total=42"));

    let saved_state = fs::read_to_string(&state_out).unwrap();
    assert!(saved_state.contains("invoice.status=Finalized"));
    assert!(saved_state.contains("invoice.subtotal=40"));
    assert!(saved_state.contains("invoice.tax=2"));
    assert!(saved_state.contains("invoice.total=42"));

    let _ = fs::remove_file(state_in);
    let _ = fs::remove_file(state_out);
}

#[test]
fn cli_dispatch_routes_requests_through_endpoints() {
    let binary = env!("CARGO_BIN_EXE_eigl");
    let state_in = temp_path("dispatch-in");
    let state_out = temp_path("dispatch-out");
    let list_state_in = temp_path("dispatch-list-in");
    let data_in = fixture("ticket.data");
    let data_out = temp_path("dispatch-data-out");
    let delete_data_out = temp_path("dispatch-delete-data-out");
    fs::write(
        &state_in,
        "ticket.status=New\nauth.expected_token=demo-token\n",
    )
    .unwrap();

    let create = Command::new(binary)
        .args([
            "dispatch",
            "examples/ticket_api_app.rif.md",
            "POST",
            "/tickets/INC-42",
            "--state-in",
            state_in.to_str().unwrap(),
            "--data-in",
            data_in.as_str(),
            "--state-out",
            state_out.to_str().unwrap(),
            "--data-out",
            data_out.to_str().unwrap(),
            "id=INC-42",
            "title=Printer is jammed",
            "assignee=Sam",
            "tags[0].name=printer",
            "auth.bearer_token=demo-token",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(create.status.success());
    let stdout = String::from_utf8_lossy(&create.stdout);
    assert!(stdout.contains("dispatch succeeded"));
    assert!(stdout.contains("ticket.id=INC-42"));
    assert!(stdout.contains("ticket.status=Open"));
    assert!(stdout.contains("ticket.title=Printer is jammed"));
    assert!(stdout.contains("ticket.assignee=Sam"));
    assert!(stdout.contains("outputs=ticket_id,ticket_status"));
    assert!(stdout.contains("response.id=INC-42"));
    assert!(stdout.contains("response.status=Open"));
    assert!(stdout.contains("response.created=true"));
    assert!(stdout.contains("response.total=1"));
    assert!(stdout.contains(r#"response.meta={"kind":"ticket"}"#));
    assert!(stdout.contains("http_status=201 Created"));

    let saved_state = fs::read_to_string(&state_out).unwrap();
    assert!(saved_state.contains("ticket.id=INC-42"));
    assert!(saved_state.contains("ticket.status=Open"));
    assert!(saved_state.contains("ticket.title=Printer is jammed"));
    assert!(saved_state.contains("ticket.assignee=Sam"));
    assert!(saved_state.contains("tickets.INC-42.id=INC-42"));
    assert!(saved_state.contains("tickets.INC-42.status=Open"));
    assert!(saved_state.contains("tickets.INC-42.title=Printer is jammed"));
    assert!(saved_state.contains("tickets.INC-42.assignee=Sam"));

    let saved_data = fs::read_to_string(&data_out).unwrap();
    assert!(saved_data.contains("tickets.INC-42.status=Open"));
    assert!(saved_data.contains("tickets.INC-42.title=Printer is jammed"));

    fs::write(&list_state_in, "auth.expected_token=demo-token\n").unwrap();
    let list = Command::new(binary)
        .args([
            "dispatch",
            "examples/ticket_api_app.rif.md",
            "GET",
            "/tickets",
            "--state-in",
            list_state_in.to_str().unwrap(),
            "--data-in",
            data_out.to_str().unwrap(),
            "auth.bearer_token=demo-token",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(
        list.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&list.stdout),
        String::from_utf8_lossy(&list.stderr)
    );
    let list_stdout = String::from_utf8_lossy(&list.stdout);
    assert!(list_stdout.contains("dispatch succeeded"));
    assert!(list_stdout.contains("response.total=1"));
    assert!(list_stdout.contains(r#"response.ids=["INC-42"]"#));
    assert!(list_stdout.contains("response.items=[{"));
    assert!(list_stdout.contains(r#""id":"INC-42""#));
    assert!(list_stdout.contains(r#""title":"Printer is jammed""#));

    let filtered_list = Command::new(binary)
        .args([
            "dispatch",
            "examples/ticket_api_app.rif.md",
            "GET",
            "/tickets/status/Open",
            "--state-in",
            list_state_in.to_str().unwrap(),
            "--data-in",
            data_out.to_str().unwrap(),
            "auth.bearer_token=demo-token",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(
        filtered_list.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&filtered_list.stdout),
        String::from_utf8_lossy(&filtered_list.stderr)
    );
    let filtered_stdout = String::from_utf8_lossy(&filtered_list.stdout);
    assert!(filtered_stdout.contains("dispatch succeeded"));
    assert!(filtered_stdout.contains("response.total=1"));
    assert!(filtered_stdout.contains(r#"response.ids=["INC-42"]"#));
    assert!(filtered_stdout.contains("response.items=[{"));
    assert!(filtered_stdout.contains(r#""status":"Open""#));

    let missing = Command::new(binary)
        .args([
            "dispatch",
            "examples/ticket_api_app.rif.md",
            "GET",
            "/tickets/MISSING",
            "--state-in",
            list_state_in.to_str().unwrap(),
            "--data-in",
            data_out.to_str().unwrap(),
            "auth.bearer_token=demo-token",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(!missing.status.success());
    let missing_stdout = String::from_utf8_lossy(&missing.stdout);
    assert!(missing_stdout.contains("failure=NotFound"));
    assert!(missing_stdout.contains("error.code=NotFound"));
    assert!(missing_stdout.contains("http_status=404 Not Found"));

    let delete = Command::new(binary)
        .args([
            "dispatch",
            "examples/ticket_api_app.rif.md",
            "DELETE",
            "/tickets/INC-42",
            "--state-in",
            state_out.to_str().unwrap(),
            "--data-in",
            data_out.to_str().unwrap(),
            "--state-out",
            state_out.to_str().unwrap(),
            "--data-out",
            delete_data_out.to_str().unwrap(),
            "id=INC-42",
            "auth.bearer_token=demo-token",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(delete.status.success());
    let delete_stdout = String::from_utf8_lossy(&delete.stdout);
    assert!(delete_stdout.contains("dispatch succeeded"));
    assert!(delete_stdout.contains("ticket.id=INC-42"));
    assert!(delete_stdout.contains("ticket.status=Open"));

    let deleted_data = fs::read_to_string(&delete_data_out).unwrap();
    assert!(!deleted_data.contains("tickets.INC-42.status"));
    assert!(!deleted_data.contains("tickets.INC-42.title"));

    let _ = fs::remove_file(state_in);
    let _ = fs::remove_file(state_out);
    let _ = fs::remove_file(list_state_in);
    let _ = fs::remove_file(data_out);
    let _ = fs::remove_file(delete_data_out);
}

#[test]
fn cli_dispatch_binds_aggregate_object_request_fields() {
    let binary = env!("CARGO_BIN_EXE_eigl");
    let source = temp_path("aggregate-object-dispatch").with_extension("rif.md");
    fs::write(
        &source,
        r#"
        app AggregateObjectDispatch

        things:
          thing Ticket
            field id: Text
            field title: Text
            field estimate: Int

        endpoints:
          endpoint POST /tickets -> StoreTicket
            request:
              ticket: Ticket
            bind:
              ticket = ticket
            respond:
              ticket: Ticket
              ticket = ticket

        intent StoreTicket

        subject:
          ticket: Ticket
        "#,
    )
    .unwrap();

    let output = Command::new(binary)
        .args([
            "dispatch",
            source.to_str().unwrap(),
            "POST",
            "/tickets",
            r#"ticket={"id":"T-1","title":"Printer","estimate":3}"#,
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(r#"response.ticket={"estimate":3,"id":"T-1","title":"Printer"}"#));

    let _ = fs::remove_file(source);
}

#[test]
fn cli_dispatch_binds_collection_records_as_typed_objects() {
    let binary = env!("CARGO_BIN_EXE_eigl");
    let source = temp_path("collection-object-binding").with_extension("rif.md");
    let data_in = temp_path("collection-object-binding-data");
    fs::write(
        &source,
        r#"
        app CollectionObjectBinding

        things:
          thing Ticket
            field id: Text
            field title: Text
            field status: State<Open, Closed>

        collections:
          collection tickets: Ticket

        endpoints:
          endpoint GET /tickets/{id} -> ShowTicket
            requires:
              tickets[id].id exists else NotFound
            bind:
              ticket = tickets[id]
            respond:
              ticket: Ticket
              ticket = ticket

        intent ShowTicket

        subject:
          ticket: Ticket
        "#,
    )
    .unwrap();
    fs::write(
        &data_in,
        "tickets.T-1.id=T-1\ntickets.T-1.title=Printer\ntickets.T-1.status=Open\n",
    )
    .unwrap();

    let output = Command::new(binary)
        .args([
            "dispatch",
            source.to_str().unwrap(),
            "GET",
            "/tickets/T-1",
            "--data-in",
            data_in.to_str().unwrap(),
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(r#"response.ticket={"id":"T-1","status":"Open","title":"Printer"}"#));

    let _ = fs::remove_file(source);
    let _ = fs::remove_file(data_in);
}

#[test]
fn cli_dispatch_uses_named_endpoint_requirement_failures() {
    let binary = env!("CARGO_BIN_EXE_eigl");
    let source = temp_path("named-endpoint-error").with_extension("rif.md");
    let state_in = temp_path("named-endpoint-error-state");
    let data_in = temp_path("named-endpoint-error-data");
    fs::write(
        &source,
        r#"
        app TicketApi

        things:
          thing Ticket
            field id: Text
            field status: State<Open, Closed>

        collections:
          collection tickets: Ticket

        endpoints:
          endpoint GET /tickets/{id} -> ShowTicket
            requires:
              auth.bearer_token is auth.expected_token
              tickets[id].id exists else NotFound
            respond:
              item = tickets[id].record_json
              id = tickets[id].id
              status = tickets[id].status
            error NotFound:
              status: 404 Not Found
              code = failure
              message = "ticket not found"

        intent ShowTicket
        "#,
    )
    .unwrap();
    fs::write(&state_in, "auth.expected_token=demo-token\n").unwrap();
    fs::write(&data_in, "tickets.T1.id=T1\ntickets.T1.status=Open\n").unwrap();

    let output = Command::new(binary)
        .args([
            "dispatch",
            source.to_str().unwrap(),
            "GET",
            "/tickets/MISSING",
            "--state-in",
            state_in.to_str().unwrap(),
            "--data-in",
            data_in.to_str().unwrap(),
            "auth.bearer_token=demo-token",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("failure=NotFound"));
    assert!(stdout.contains("error.code=NotFound"));
    assert!(stdout.contains("error.message=ticket not found"));
    assert!(stdout.contains("http_status=404 Not Found"));

    let existing = Command::new(binary)
        .args([
            "dispatch",
            source.to_str().unwrap(),
            "GET",
            "/tickets/T1",
            "--state-in",
            state_in.to_str().unwrap(),
            "--data-in",
            data_in.to_str().unwrap(),
            "auth.bearer_token=demo-token",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();

    assert!(
        existing.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&existing.stdout),
        String::from_utf8_lossy(&existing.stderr)
    );
    let existing_stdout = String::from_utf8_lossy(&existing.stdout);
    assert!(existing_stdout.contains(r#"response.item={"id":"T1","status":"Open"}"#));
    assert!(existing_stdout.contains("response.id=T1"));
    assert!(existing_stdout.contains("response.status=Open"));

    let _ = fs::remove_file(source);
    let _ = fs::remove_file(state_in);
    let _ = fs::remove_file(data_in);
}

#[test]
fn cli_dispatch_rejects_invalid_typed_endpoint_request_fields() {
    let binary = env!("CARGO_BIN_EXE_eigl");
    let source = temp_path("typed-endpoint-request").with_extension("rif.md");
    fs::write(
        &source,
        r#"
        app TypedRequestApi

        things:
          thing Ticket
            field title: Text
            field estimate: Int

        endpoints:
          endpoint POST /tickets -> CreateTicket
            request:
              title: Text
              estimate: Int
            bind:
              ticket.title = title
              ticket.estimate = estimate
            respond:
              title = ticket.title
              estimate = ticket.estimate
            error BadRequest:
              status: 400 Bad Request
              code = failure
              message = failure

        intent CreateTicket

        subject:
          ticket: Ticket

        steps:
          1. Keep ticket
             set: ticket.title = ticket.title
        "#,
    )
    .unwrap();

    let invalid = Command::new(binary)
        .args([
            "dispatch",
            source.to_str().unwrap(),
            "POST",
            "/tickets",
            "title=Printer",
            "estimate=soon",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(!invalid.status.success());
    let invalid_stdout = String::from_utf8_lossy(&invalid.stdout);
    assert!(invalid_stdout.contains("failure=BadRequest"));
    assert!(invalid_stdout.contains("error.code=BadRequest"));
    assert!(invalid_stdout.contains("http_status=400 Bad Request"));

    let missing = Command::new(binary)
        .args([
            "dispatch",
            source.to_str().unwrap(),
            "POST",
            "/tickets",
            "estimate=3",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(!missing.status.success());
    let missing_stdout = String::from_utf8_lossy(&missing.stdout);
    assert!(missing_stdout.contains("failure=BadRequest"));

    let valid = Command::new(binary)
        .args([
            "dispatch",
            source.to_str().unwrap(),
            "POST",
            "/tickets",
            "title=Printer",
            "estimate=3",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(
        valid.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&valid.stdout),
        String::from_utf8_lossy(&valid.stderr)
    );
    let valid_stdout = String::from_utf8_lossy(&valid.stdout);
    assert!(valid_stdout.contains("response.title=Printer"));
    assert!(valid_stdout.contains("response.estimate=3"));

    let _ = fs::remove_file(source);
}

#[test]
fn cli_emit_routes_events_through_triggers() {
    let binary = env!("CARGO_BIN_EXE_eigl");
    let state_in = temp_path("emit-in");
    let state_out = temp_path("emit-out");
    let data_in = fixture("ticket.data");
    let data_out = temp_path("emit-data-out");
    fs::write(&state_in, "ticket.status=New\n").unwrap();

    let emit = Command::new(binary)
        .args([
            "emit",
            "examples/ticket_event_app.rif.md",
            "ticket.created",
            "--state-in",
            state_in.to_str().unwrap(),
            "--data-in",
            data_in.as_str(),
            "--state-out",
            state_out.to_str().unwrap(),
            "--data-out",
            data_out.to_str().unwrap(),
            "id=INC-77",
            "title=Printer jammed",
            "assignee=Sam",
            "tags[0].name=printer",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(emit.status.success());
    let stdout = String::from_utf8_lossy(&emit.stdout);
    assert!(stdout.contains("emit succeeded"));
    assert!(stdout.contains("ticket.id=INC-77"));
    assert!(stdout.contains("ticket.status=Open"));
    assert!(stdout.contains("ticket.primary_tag=printer"));
    assert!(stdout.contains("outputs=ticket_id,ticket_status"));

    let saved_state = fs::read_to_string(&state_out).unwrap();
    assert!(saved_state.contains("ticket.id=INC-77"));
    assert!(saved_state.contains("ticket.status=Open"));
    assert!(saved_state.contains("ticket.primary_tag=printer"));

    let saved_data = fs::read_to_string(&data_out).unwrap();
    assert!(saved_data.contains("tickets.INC-77.status=Open"));
    assert!(saved_data.contains("tickets.INC-77.primary_tag=printer"));

    let _ = fs::remove_file(state_in);
    let _ = fs::remove_file(state_out);
    let _ = fs::remove_file(data_out);
}

#[test]
fn cli_emit_rejects_invalid_typed_trigger_payload_fields() {
    let binary = env!("CARGO_BIN_EXE_eigl");
    let source = temp_path("typed-trigger-payload").with_extension("rif.md");
    fs::write(
        &source,
        r#"
        app TypedEventApi

        things:
          thing Job
            field id: Text
            field run_index: Int

        triggers:
          trigger nightly.cleanup -> RunCleanup
            payload:
              job_id: Text
              run_index: Int
            bind:
              job.id = event.job_id
              job.run_index = event.run_index

        intent RunCleanup

        subject:
          job: Job

        steps:
          1. Record run
             set: job.run_index = job.run_index + 1
        "#,
    )
    .unwrap();

    let invalid = Command::new(binary)
        .args([
            "emit",
            source.to_str().unwrap(),
            "nightly.cleanup",
            "job_id=nightly",
            "run_index=soon",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(!invalid.status.success());
    let invalid_stdout = String::from_utf8_lossy(&invalid.stdout);
    assert!(invalid_stdout.contains("emit failed"));
    assert!(invalid_stdout.contains("failure=BadEvent"));

    let missing = Command::new(binary)
        .args([
            "emit",
            source.to_str().unwrap(),
            "nightly.cleanup",
            "run_index=1",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(!missing.status.success());
    let missing_stdout = String::from_utf8_lossy(&missing.stdout);
    assert!(missing_stdout.contains("failure=BadEvent"));

    let valid = Command::new(binary)
        .args([
            "emit",
            source.to_str().unwrap(),
            "nightly.cleanup",
            "job_id=nightly",
            "run_index=1",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(
        valid.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&valid.stdout),
        String::from_utf8_lossy(&valid.stderr)
    );
    let valid_stdout = String::from_utf8_lossy(&valid.stdout);
    assert!(valid_stdout.contains("emit succeeded"));
    assert!(valid_stdout.contains("job.id=nightly"));
    assert!(valid_stdout.contains("job.run_index=2"));

    let _ = fs::remove_file(source);
}

#[test]
fn cli_dequeue_routes_queue_messages_through_triggers() {
    let binary = env!("CARGO_BIN_EXE_eigl");
    let state_in = temp_path("queue-in");
    let state_out = temp_path("queue-out");
    fs::write(&state_in, "").unwrap();

    let dequeue = Command::new(binary)
        .args([
            "dequeue",
            "examples/queue_inbox_app.rif.md",
            "support.inbox",
            "--state-in",
            state_in.to_str().unwrap(),
            "--state-out",
            state_out.to_str().unwrap(),
            "message_id=msg-1",
            "message=hello from queue",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(dequeue.status.success());
    let stdout = String::from_utf8_lossy(&dequeue.stdout);
    assert!(stdout.contains("dequeue succeeded"));
    assert!(stdout.contains("inbox.id=msg-1"));
    assert!(stdout.contains("inbox.last_message=hello from queue"));

    let saved_state = fs::read_to_string(&state_out).unwrap();
    assert!(saved_state.contains("inbox.id=msg-1"));
    assert!(saved_state.contains("inbox.last_message=hello from queue"));
    assert!(saved_state.contains("inboxes.msg-1.id=msg-1"));
    assert!(saved_state.contains("inboxes.msg-1.last_message=hello from queue"));

    let _ = fs::remove_file(state_in);
    let _ = fs::remove_file(state_out);
}

#[test]
fn cli_serve_uses_cookie_sessions_for_endpoint_guards() {
    let _server_guard = server_test_guard();
    let binary = env!("CARGO_BIN_EXE_eigl");
    let state_in = temp_path("session-in");
    let state_out = temp_path("session-out");
    let data_out = temp_path("session-data-out");
    fs::write(&state_in, "auth.expected_session=demo-session\n").unwrap();

    let listen = free_listen_addr();
    let mut child = Command::new(binary)
        .args([
            "serve",
            "examples/session_inbox_app.rif.md",
            "--listen",
            listen.as_str(),
            "--state-in",
            state_in.to_str().unwrap(),
            "--state-out",
            state_out.to_str().unwrap(),
            "--data-out",
            data_out.to_str().unwrap(),
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let stderr = child.stderr.take().unwrap();
    let mut stderr = BufReader::new(stderr);
    let mut startup_log = String::new();
    let listen = loop {
        let mut line = String::new();
        let read = stderr.read_line(&mut line).unwrap();
        if read == 0 {
            break None;
        }
        startup_log.push_str(&line);
        if let Some(value) = line.trim().strip_prefix("listening on ") {
            break Some(value.to_string());
        }
    }
    .unwrap_or_else(|| panic!("server never reported bind address:\n{startup_log}"));

    let body = r#"{"message":"session hello"}"#;
    let unauthorized = format!(
        "POST /inbox HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    let mut stream = TcpStream::connect(&listen).unwrap();
    stream.write_all(unauthorized.as_bytes()).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    assert!(response.contains("401 Unauthorized"));
    assert!(response.contains("failure=Unauthorized"));
    assert!(!state_out.exists());
    assert!(!data_out.exists());

    let authorized = format!(
        "POST /inbox HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nCookie: session=demo-session\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    let mut stream = TcpStream::connect(&listen).unwrap();
    stream.write_all(authorized.as_bytes()).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    assert!(response.contains("200 OK"));
    assert!(response.contains("dispatch succeeded"));
    assert!(response.contains("inbox.id=demo-session"));
    assert!(response.contains("inbox.last_message=session hello"));

    let saved_state = fs::read_to_string(&state_out).unwrap();
    assert!(saved_state.contains("inbox.id=demo-session"));
    assert!(saved_state.contains("inbox.last_message=session hello"));
    assert!(saved_state.contains("inboxes.demo-session.id=demo-session"));
    assert!(saved_state.contains("inboxes.demo-session.last_message=session hello"));

    let saved_data = fs::read_to_string(&data_out).unwrap();
    assert!(saved_data.contains("inboxes.demo-session.last_message=session hello"));

    let _ = child.kill();
    let _ = child.wait();
    let _ = fs::remove_file(state_in);
    let _ = fs::remove_file(state_out);
    let _ = fs::remove_file(data_out);
}

#[test]
fn cli_schedule_routes_jobs_through_triggers() {
    let binary = env!("CARGO_BIN_EXE_eigl");
    let state_in = temp_path("schedule-in");
    let state_out = temp_path("schedule-out");
    fs::write(&state_in, "job.id=cleanup-1\njob.runs=0\n").unwrap();

    let schedule = Command::new(binary)
        .args([
            "schedule",
            "examples/daily_cleanup_app.rif.md",
            "nightly.cleanup",
            "--state-in",
            state_in.to_str().unwrap(),
            "--state-out",
            state_out.to_str().unwrap(),
            "job_id=cleanup-1",
            "run_at=2026-05-20T09:30:00Z",
            "run_index=1",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(schedule.status.success());
    let stdout = String::from_utf8_lossy(&schedule.stdout);
    assert!(stdout.contains("schedule succeeded"));
    assert!(stdout.contains("job.id=cleanup-1"));
    assert!(stdout.contains("job.last_run=2026-05-20T09:30:00Z"));
    assert!(stdout.contains("job.runs=2"));
    assert!(stdout.contains("outputs=job_id,job_runs"));

    let saved_state = fs::read_to_string(&state_out).unwrap();
    assert!(saved_state.contains("job.id=cleanup-1"));
    assert!(saved_state.contains("job.last_run=2026-05-20T09:30:00Z"));
    assert!(saved_state.contains("job.runs=2"));

    let _ = fs::remove_file(state_in);
    let _ = fs::remove_file(state_out);
}

#[test]
fn cli_serve_routes_http_requests_through_endpoints() {
    let _server_guard = server_test_guard();
    let binary = env!("CARGO_BIN_EXE_eigl");
    let state_in = temp_path("serve-in");
    let state_out = temp_path("serve-out");
    let data_in = fixture("ticket.data");
    let data_out = temp_path("serve-data-out");
    fs::write(
        &state_in,
        "ticket.status=New\nauth.expected_token=demo-token\n",
    )
    .unwrap();

    let listen = free_listen_addr();
    let mut child = Command::new(binary)
        .args([
            "serve",
            "examples/ticket_api_app.rif.md",
            "--listen",
            listen.as_str(),
            "--state-in",
            state_in.to_str().unwrap(),
            "--data-in",
            data_in.as_str(),
            "--state-out",
            state_out.to_str().unwrap(),
            "--data-out",
            data_out.to_str().unwrap(),
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let stderr = child.stderr.take().unwrap();
    let mut stderr = BufReader::new(stderr);
    let mut startup_log = String::new();
    let listen = loop {
        let mut line = String::new();
        let read = stderr.read_line(&mut line).unwrap();
        if read == 0 {
            break None;
        }
        startup_log.push_str(&line);
        if let Some(value) = line.trim().strip_prefix("listening on ") {
            break Some(value.to_string());
        }
    }
    .unwrap_or_else(|| panic!("server never reported bind address:\n{startup_log}"));

    let mut connected = false;
    for _ in 0..50 {
        if TcpStream::connect(&listen).is_ok() {
            connected = true;
            break;
        }
        thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(connected, "server never started");

    let body = r#"{"id":"INC-99","title":"Printer jammed","assignee":"Sam","tags":[{"name":"printer"},{"name":"jammed"}]}"#;

    let mut unauthorized = TcpStream::connect(&listen).unwrap();
    let unauthorized_request = format!(
        "POST /tickets/INC-99 HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nX-Request-Id: req-123\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    unauthorized
        .write_all(unauthorized_request.as_bytes())
        .unwrap();
    let mut unauthorized_response = String::new();
    unauthorized
        .read_to_string(&mut unauthorized_response)
        .unwrap();
    assert!(unauthorized_response.contains("401 Unauthorized"));
    assert!(unauthorized_response.contains("Content-Type: application/json"));
    assert!(unauthorized_response.contains(r#""code":"Unauthorized""#));
    assert!(unauthorized_response.contains(r#""message":"Unauthorized""#));
    assert!(!state_out.exists());
    assert!(!data_out.exists());

    let mut stream = TcpStream::connect(&listen).unwrap();
    let request = format!(
        "POST /tickets/INC-99 HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nAuthorization: Bearer demo-token\r\nX-Request-Id: req-123\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(request.as_bytes()).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    assert!(response.contains("201 Created"));
    assert!(response.contains("Content-Type: application/json"));
    assert!(response.contains(r#""id":"INC-99""#));
    assert!(response.contains(r#""status":"Open""#));
    assert!(response.contains(r#""tag":"printer""#));
    assert!(response.contains(r#""request_id":"req-123""#));
    assert!(response.contains(r#""created":true"#));
    assert!(response.contains(r#""total":2"#));
    assert!(response.contains(r#""meta":{"kind":"ticket"}"#));

    let saved_state = fs::read_to_string(&state_out).unwrap();
    assert!(saved_state.contains("ticket.id=INC-99"));
    assert!(saved_state.contains("ticket.status=Open"));
    assert!(saved_state.contains("ticket.primary_tag=printer"));
    assert!(saved_state.contains("ticket.request_id=req-123"));

    let saved_data = fs::read_to_string(&data_out).unwrap();
    assert!(saved_data.contains("tickets.INC-99.status=Open"));
    assert!(saved_data.contains("tickets.INC-99.primary_tag=printer"));
    assert!(saved_data.contains("tickets.INC-99.request_id=req-123"));

    let _ = child.kill();
    let _ = child.wait();
    let _ = fs::remove_file(state_in);
    let _ = fs::remove_file(state_out);
    let _ = fs::remove_file(data_out);
}

#[test]
fn cli_serve_binds_json_array_and_object_request_fields() {
    let _server_guard = server_test_guard();
    let binary = env!("CARGO_BIN_EXE_eigl");
    let source = temp_path("json-aggregate-request").with_extension("rif.md");
    fs::write(
        &source,
        r#"
        app JsonAggregateApi

        things:
          thing Message
            field tags: List<Text>
            field metadata: Map<Text, Text>

        endpoints:
          endpoint POST /messages -> StoreMessage
            request:
              tags: List<Text>
              metadata: Map<Text, Text>
            bind:
              message.tags = tags
              message.metadata = metadata
            respond:
              tags: List<Text>
              metadata: Map<Text, Text>
              first_tag: Text
              channel: Text
              tags = message.tags
              metadata = message.metadata
              first_tag = message.tags[0]
              channel = message.metadata["channel"]
            error BadRequest:
              status: 400 Bad Request
              code = failure

        intent StoreMessage

        subject:
          message: Message

        steps:
          1. Keep message
             set: message.tags = message.tags
             set: message.metadata = message.metadata
        "#,
    )
    .unwrap();

    let listen = free_listen_addr();
    let mut child = Command::new(binary)
        .args([
            "serve",
            source.to_str().unwrap(),
            "--listen",
            listen.as_str(),
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let stderr = child.stderr.take().unwrap();
    let mut stderr = BufReader::new(stderr);
    let mut startup_log = String::new();
    let listen = loop {
        let mut line = String::new();
        let read = stderr.read_line(&mut line).unwrap();
        if read == 0 {
            break None;
        }
        startup_log.push_str(&line);
        if let Some(value) = line.trim().strip_prefix("listening on ") {
            break Some(value.to_string());
        }
    }
    .unwrap_or_else(|| panic!("server never reported bind address:\n{startup_log}"));

    let body = r#"{"tags":["urgent","vip"],"metadata":{"channel":"support","source":"web"}}"#;
    let request = format!(
        "POST /messages HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    let mut stream = TcpStream::connect(&listen).unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();

    assert!(response.contains("200 OK"), "{response}");
    assert!(response.contains(r#""tags":["urgent","vip"]"#));
    assert!(response.contains(r#""metadata":{"channel":"support","source":"web"}"#));
    assert!(response.contains(r#""first_tag":"urgent""#));
    assert!(response.contains(r#""channel":"support""#));

    let _ = child.kill();
    let _ = child.wait();
    let _ = fs::remove_file(source);
}

#[test]
fn cli_serve_binds_typed_json_object_request_fields() {
    let _server_guard = server_test_guard();
    let binary = env!("CARGO_BIN_EXE_eigl");
    let source = temp_path("json-object-request").with_extension("rif.md");
    fs::write(
        &source,
        r#"
        app JsonObjectApi

        things:
          thing Ticket
            field id: Text
            field title: Text
            field estimate: Int

        endpoints:
          endpoint POST /tickets -> StoreTicket
            request:
              ticket: Ticket
            bind:
              ticket = ticket
            respond:
              id: Text
              title: Text
              estimate: Int
              id = ticket.id
              title = ticket.title
              estimate = ticket.estimate
            error BadRequest:
              status: 400 Bad Request
              code = failure

        intent StoreTicket

        subject:
          ticket: Ticket

        steps:
          1. Keep ticket
             set: ticket.title = ticket.title
        "#,
    )
    .unwrap();

    let listen = free_listen_addr();
    let mut child = Command::new(binary)
        .args([
            "serve",
            source.to_str().unwrap(),
            "--listen",
            listen.as_str(),
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let stderr = child.stderr.take().unwrap();
    let mut stderr = BufReader::new(stderr);
    let mut startup_log = String::new();
    let listen = loop {
        let mut line = String::new();
        let read = stderr.read_line(&mut line).unwrap();
        if read == 0 {
            break None;
        }
        startup_log.push_str(&line);
        if let Some(value) = line.trim().strip_prefix("listening on ") {
            break Some(value.to_string());
        }
    }
    .unwrap_or_else(|| panic!("server never reported bind address:\n{startup_log}"));

    let bad_body = r#"{"ticket":{"id":"T-1","title":"Printer","estimate":"soon"}}"#;
    let bad_request = format!(
        "POST /tickets HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        bad_body.len(),
        bad_body
    );
    let mut stream = TcpStream::connect(&listen).unwrap();
    stream.write_all(bad_request.as_bytes()).unwrap();
    let mut bad_response = String::new();
    stream.read_to_string(&mut bad_response).unwrap();
    assert!(bad_response.contains("400 Bad Request"), "{bad_response}");
    assert!(bad_response.contains(r#""code":"BadRequest""#));

    let body = r#"{"ticket":{"id":"T-1","title":"Printer","estimate":3}}"#;
    let request = format!(
        "POST /tickets HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    let mut stream = TcpStream::connect(&listen).unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();

    assert!(response.contains("200 OK"), "{response}");
    assert!(response.contains(r#""id":"T-1""#));
    assert!(response.contains(r#""title":"Printer""#));
    assert!(response.contains(r#""estimate":3"#));

    let _ = child.kill();
    let _ = child.wait();
    let _ = fs::remove_file(source);
}

#[test]
fn cli_serve_returns_typed_object_response_fields() {
    let _server_guard = server_test_guard();
    let binary = env!("CARGO_BIN_EXE_eigl");
    let source = temp_path("object-response").with_extension("rif.md");
    let state_in = temp_path("object-response-state");
    fs::write(
        &source,
        r#"
        app ObjectResponseApi

        things:
          thing Ticket
            field id: Text
            field title: Text
            field estimate: Int

        endpoints:
          endpoint GET /ticket -> ShowTicket
            respond:
              ticket: Ticket
              ticket = ticket

        intent ShowTicket

        subject:
          ticket: Ticket
        "#,
    )
    .unwrap();
    fs::write(
        &state_in,
        "ticket.id=T-1\nticket.title=Printer\nticket.estimate=3\n",
    )
    .unwrap();

    let listen = free_listen_addr();
    let mut child = Command::new(binary)
        .args([
            "serve",
            source.to_str().unwrap(),
            "--listen",
            listen.as_str(),
            "--state-in",
            state_in.to_str().unwrap(),
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let stderr = child.stderr.take().unwrap();
    let mut stderr = BufReader::new(stderr);
    let mut startup_log = String::new();
    let listen = loop {
        let mut line = String::new();
        let read = stderr.read_line(&mut line).unwrap();
        if read == 0 {
            break None;
        }
        startup_log.push_str(&line);
        if let Some(value) = line.trim().strip_prefix("listening on ") {
            break Some(value.to_string());
        }
    }
    .unwrap_or_else(|| panic!("server never reported bind address:\n{startup_log}"));

    let request = "GET /ticket HTTP/1.1\r\nHost: localhost\r\n\r\n";
    let mut stream = TcpStream::connect(&listen).unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();

    assert!(response.contains("200 OK"), "{response}");
    assert!(response.contains(r#""ticket":{"estimate":3,"id":"T-1","title":"Printer"}"#));

    let _ = child.kill();
    let _ = child.wait();
    let _ = fs::remove_file(source);
    let _ = fs::remove_file(state_in);
}

#[test]
fn cli_simulate_accepts_runtime_state_assignments() {
    let binary = env!("CARGO_BIN_EXE_eigl");

    let simulate = Command::new(binary)
        .args([
            "simulate",
            "examples/confirm_order.rif.md",
            "order.status=Draft",
            "order.items.count=2",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(simulate.status.success());
    let stdout = String::from_utf8_lossy(&simulate.stdout);
    assert!(stdout.contains("succeeded"));
    assert!(stdout.contains("order.status=Confirmed"));
}

#[test]
fn cli_run_reports_missing_runtime_requirement() {
    let binary = env!("CARGO_BIN_EXE_eigl");

    let run_invoice = Command::new(binary)
        .args([
            "run",
            "examples/invoice_app.rif.md",
            "invoice.status=Draft",
            "invoice.subtotal=40",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert_eq!(run_invoice.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&run_invoice.stdout);
    assert!(stdout.contains("bytecode failed"));
    assert!(stdout.contains("failure=RequirementFailed"));
    assert!(stdout.contains("CHECK FAILED invoice.tax > 0"));
}

#[test]
fn cli_rejects_unknown_runtime_state_field_for_declared_app() {
    let binary = env!("CARGO_BIN_EXE_eigl");

    let run_invoice = Command::new(binary)
        .args([
            "run",
            "examples/invoice_app.rif.md",
            "invoice.status=Draft",
            "invoice.subttoal=40",
            "invoice.tax=2",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert_eq!(run_invoice.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&run_invoice.stderr)
            .contains("unknown runtime state 'invoice.subttoal'")
    );
}

#[test]
fn cli_validates_nested_runtime_state_paths_for_declared_app() {
    let binary = env!("CARGO_BIN_EXE_eigl");

    let bad_nested_int = Command::new(binary)
        .args(["run", "examples/profile_app.rif.md", "user.profile.age=old"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert_eq!(bad_nested_int.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&bad_nested_int.stderr)
            .contains("invalid runtime value for 'user.profile.age': expected Int")
    );

    let bad_nested_field = Command::new(binary)
        .args(["run", "examples/profile_app.rif.md", "user.profile.agge=18"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert_eq!(bad_nested_field.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&bad_nested_field.stderr).contains(
            "unknown runtime state 'user.profile.agge': type 'Profile' has no field 'agge'"
        )
    );
}

#[test]
fn cli_rejects_runtime_state_list_values_with_invalid_elements() {
    let binary = env!("CARGO_BIN_EXE_eigl");

    let bad_list = Command::new(binary)
        .args([
            "run",
            "examples/bulk_import_app.rif.md",
            "batch.counts=[1,two,3]",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert_eq!(bad_list.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&bad_list.stderr)
            .contains("invalid runtime value for 'batch.counts': expected List<Int>")
    );
}

#[test]
fn cli_rejects_runtime_state_map_values_with_invalid_entries() {
    let binary = env!("CARGO_BIN_EXE_eigl");

    let bad_map = Command::new(binary)
        .args([
            "run",
            "examples/dashboard_app.rif.md",
            "dashboard.counts={\"open\":1,\"closed\":two}",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert_eq!(bad_map.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&bad_map.stderr)
            .contains("invalid runtime value for 'dashboard.counts': expected Map<Text, Int>")
    );
}

#[test]
fn cli_rejects_runtime_state_option_values_with_invalid_inner_value() {
    let binary = env!("CARGO_BIN_EXE_eigl");

    let bad_option = Command::new(binary)
        .args([
            "run",
            "examples/optional_profile_app.rif.md",
            "user.age=Some(old)",
            "user.nickname=None",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert_eq!(bad_option.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&bad_option.stderr)
            .contains("invalid runtime value for 'user.age': expected Option<Int>")
    );
}

#[test]
fn cli_rejects_runtime_state_result_values_with_invalid_inner_value() {
    let binary = env!("CARGO_BIN_EXE_eigl");

    let bad_result = Command::new(binary)
        .args([
            "run",
            "examples/result_payment_app.rif.md",
            "payment.confirmation=Success(old)",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert_eq!(bad_result.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&bad_result.stderr).contains(
            "invalid runtime value for 'payment.confirmation': expected Result<Int, Text>"
        )
    );
}

#[test]
fn cli_uses_supplied_operation_output_values() {
    let binary = env!("CARGO_BIN_EXE_eigl");
    let document_path = temp_path("operation-output-values.rif.md");
    fs::write(
        &document_path,
        r#"
        app OperationOutputValues

        things:
          thing Ticket
            field id: Text
            field title: Text

        operations:
          operation Catalog.title(id: Text) -> Text

        intent FillTicketTitle

        subject:
          ticket: Ticket

        steps:
          1. Fetch title
             call: Catalog.title(ticket.id)
             output: fetched_title: Text
          2. Store fetched title
             set: ticket.title = fetched_title
        "#,
    )
    .unwrap();

    let run = Command::new(binary)
        .args([
            "run",
            document_path.to_str().unwrap(),
            "--operation-output",
            "fetched_title=Printer",
            "ticket.id=T-1",
            "ticket.title=Old",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();

    assert!(
        run.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(stdout.contains("ticket.title=Printer"));
    assert!(!stdout.contains("fetched_title#1"));

    let _ = fs::remove_file(document_path);
}

#[test]
fn cli_accepts_operation_outputs_for_invoked_intents() {
    let binary = env!("CARGO_BIN_EXE_eigl");
    let document_path = temp_path("invoked-operation-output-values.rif.md");
    fs::write(
        &document_path,
        r#"
        app InvokedOperationOutputValues

        things:
          thing Ticket
            field id: Text
            field title: Text

        operations:
          operation Catalog.title(id: Text) -> Text

        intent FillTicketViaChild

        subject:
          ticket: Ticket

        steps:
          1. Fill from child
             invoke: LookupTicketTitle(ticket = ticket)

        intent LookupTicketTitle

        subject:
          ticket: Ticket

        steps:
          1. Fetch title
             call: Catalog.title(ticket.id)
             output: fetched_title: Text
          2. Store fetched title
             set: ticket.title = fetched_title
        "#,
    )
    .unwrap();

    let run = Command::new(binary)
        .args([
            "run",
            document_path.to_str().unwrap(),
            "--operation-output",
            "fetched_title=Printer",
            "ticket.id=T-1",
            "ticket.title=Old",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();

    assert!(
        run.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(stdout.contains("ticket.title=Printer"));

    let _ = fs::remove_file(document_path);
}

#[test]
fn cli_rejects_secret_runtime_state_values_with_invalid_inner_value() {
    let binary = env!("CARGO_BIN_EXE_eigl");
    let document_path = temp_path("secret-pin.rif.md");
    fs::write(
        &document_path,
        r#"
        app SecretPins

        things:
          thing Account
            field pin_hash: Text

        operations:
          operation Hash.pin(pin: Secret<Int>) -> Text

        intent SetPin

        subject:
          account: Account

        inputs:
          pin: Secret<Int>

        steps:
          1. Hash PIN
             call: Hash.pin(pin)
             output: pin_hash: Text
          2. Store PIN hash
             set: account.pin_hash = pin_hash
        "#,
    )
    .unwrap();

    let bad_secret = Command::new(binary)
        .args([
            "run",
            document_path.to_str().unwrap(),
            "account.pin_hash=old",
            "pin=abc",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert_eq!(bad_secret.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&bad_secret.stderr)
            .contains("invalid runtime value for 'pin': expected Secret<Int>")
    );

    let valid_secret = Command::new(binary)
        .args([
            "run",
            document_path.to_str().unwrap(),
            "account.pin_hash=old",
            "pin=123",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(valid_secret.status.success());
    let stdout = String::from_utf8_lossy(&valid_secret.stdout);
    assert!(stdout.contains("pin=<secret>"));
    assert!(!stdout.contains("pin=123"));

    let _ = fs::remove_file(document_path);
}

#[test]
fn cli_rejects_runtime_state_value_that_does_not_match_declared_type() {
    let binary = env!("CARGO_BIN_EXE_eigl");

    let bad_int = Command::new(binary)
        .args([
            "run",
            "examples/invoice_app.rif.md",
            "invoice.status=Draft",
            "invoice.subtotal=forty",
            "invoice.tax=2",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert_eq!(bad_int.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&bad_int.stderr)
            .contains("invalid runtime value for 'invoice.subtotal': expected Int")
    );

    let bad_state = Command::new(binary)
        .args([
            "run",
            "examples/invoice_app.rif.md",
            "invoice.status=Paid",
            "invoice.subtotal=40",
            "invoice.tax=2",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert_eq!(bad_state.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&bad_state.stderr).contains(
            "invalid runtime value for 'invoice.status': expected one of Draft, Finalized"
        )
    );
}

#[test]
fn cli_rejects_runtime_state_value_that_does_not_match_decimal() {
    let binary = env!("CARGO_BIN_EXE_eigl");

    let bad_decimal = Command::new(binary)
        .args([
            "run",
            "examples/pricing_app.rif.md",
            "product.price=twelve",
            "product.discount_rate=0.25",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert_eq!(bad_decimal.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&bad_decimal.stderr)
            .contains("invalid runtime value for 'product.price': expected Decimal")
    );
}

#[test]
fn cli_rejects_runtime_state_value_that_does_not_match_money() {
    let binary = env!("CARGO_BIN_EXE_eigl");

    let bad_money = Command::new(binary)
        .args(["run", "examples/payment_app.rif.md", "invoice.total=12.50"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert_eq!(bad_money.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&bad_money.stderr)
            .contains("invalid runtime value for 'invoice.total': expected Money")
    );
}

#[test]
fn cli_rejects_runtime_state_values_that_do_not_match_time_or_duration() {
    let binary = env!("CARGO_BIN_EXE_eigl");

    let bad_time = Command::new(binary)
        .args([
            "run",
            "examples/scheduling_app.rif.md",
            "job.starts_at=soon",
            "job.timeout=PT30M",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert_eq!(bad_time.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&bad_time.stderr)
            .contains("invalid runtime value for 'job.starts_at': expected Time")
    );

    let bad_duration = Command::new(binary)
        .args([
            "run",
            "examples/scheduling_app.rif.md",
            "job.starts_at=2026-05-20T09:30:00Z",
            "job.timeout=30M",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert_eq!(bad_duration.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&bad_duration.stderr)
            .contains("invalid runtime value for 'job.timeout': expected Duration")
    );
}

#[test]
fn cli_rejects_runtime_state_value_that_does_not_match_declared_enum() {
    let binary = env!("CARGO_BIN_EXE_eigl");

    let bad_priority = Command::new(binary)
        .args([
            "run",
            "examples/triage_app.rif.md",
            "ticket.priority=Urgent",
            "ticket.status=Open",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert_eq!(bad_priority.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&bad_priority.stderr).contains(
        "invalid runtime value for 'ticket.priority': expected one of Low, Normal, Critical"
    ));
}

#[test]
fn cli_rejects_runtime_state_value_that_does_not_match_bool() {
    let binary = env!("CARGO_BIN_EXE_eigl");

    let bad_bool = Command::new(binary)
        .args([
            "run",
            "examples/feature_flag_app.rif.md",
            "feature.enabled=yes",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert_eq!(bad_bool.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&bad_bool.stderr)
            .contains("invalid runtime value for 'feature.enabled': expected Bool")
    );
}

#[test]
fn cli_run_rejects_malformed_runtime_state_assignment() {
    let binary = env!("CARGO_BIN_EXE_eigl");

    let run_invoice = Command::new(binary)
        .args(["run", "examples/invoice_app.rif.md", "invoice.status"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert_eq!(run_invoice.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&run_invoice.stderr)
            .contains("invalid run argument 'invoice.status': expected key=value")
    );
}

#[test]
fn cli_run_selects_intent_from_multi_intent_application() {
    let binary = env!("CARGO_BIN_EXE_eigl");

    let reopen = Command::new(binary)
        .args([
            "run",
            "examples/issue_tracker_app.rif.md",
            "--intent",
            "ReopenIssue",
            "issue.status=Resolved",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(reopen.status.success());
    let stdout = String::from_utf8_lossy(&reopen.stdout);
    assert!(stdout.contains("bytecode succeeded"));
    assert!(stdout.contains("issue.status=Open"));
    assert!(stdout.contains("BEGIN Mark issue open"));
    assert!(!stdout.contains("IssueComments.add"));
}

#[test]
fn cli_reports_unknown_selected_intent() {
    let binary = env!("CARGO_BIN_EXE_eigl");

    let reopen = Command::new(binary)
        .args([
            "run",
            "examples/issue_tracker_app.rif.md",
            "--intent",
            "CloseIssue",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert_eq!(reopen.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&reopen.stderr).contains("unknown intent 'CloseIssue'"));
}
