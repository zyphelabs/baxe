#![allow(unused_variables)]

use axum::{http::StatusCode, response::IntoResponse, Json};
use baxe::{baxe_error, error, BackendError};
use std::cell::RefCell;

baxe_error!(String,);

thread_local! {
    static LOGS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

macro_rules! record {
    ($($args:tt)*) => {
        LOGS.with(|logs| logs.borrow_mut().push(format!($($args)*)))
    };
}

mod logger {
    pub(crate) use record as error;
}

#[error]
enum Visible {
    #[baxe(status = StatusCode::NOT_FOUND, tag = "missing", code = 404, message = "Missing {{item}}")]
    Unit,
    #[baxe(status = StatusCode::BAD_REQUEST, tag = "tuple", code = 400, message = "{0:04}: {1:?}")]
    Tuple(u16, Vec<usize>),
    #[baxe(status = *status, tag = "named", code = *code, message = "{0}: {1:04}")]
    Named { status: StatusCode, code: u16 },
}

#[error(logMessageWith = logger::error)]
enum Logged {
    #[baxe(status = StatusCode::BAD_REQUEST, tag = "logged", code = 400, message = "Value {0}")]
    Value(String),
}

#[error(logMessageWith=logger::error, hideMessage)]
enum HiddenLogged {
    #[baxe(status = StatusCode::BAD_REQUEST, tag = "hidden", code = 400, message = "Secret {0}")]
    Value(String),
}

#[error(hideMessage)]
enum Hidden {
    #[baxe(status = StatusCode::BAD_REQUEST, tag = "hidden", code = 400, message = "Secret {0}")]
    Value(String),
}

#[test]
fn formats_all_variant_shapes_and_preserves_metadata_bindings() {
    assert_eq!(Visible::Unit.to_string(), "Missing {item}");
    assert_eq!(Visible::Tuple(7, vec![1, 2]).to_string(), "0007: [1, 2]");
    let named = Visible::Named {
        status: StatusCode::CONFLICT,
        code: 12,
    };
    assert_eq!(named.to_string(), "409 Conflict: 0012");
    assert_eq!(named.to_status_code(), StatusCode::CONFLICT);
    assert_eq!(named.to_error_code(), 12);
    assert_eq!(named.to_error_tag().to_string(), "named");
    let response = named.into_response();
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[test]
fn logs_with_spaced_options_and_keeps_the_message() {
    let error: BaxeError = Logged::Value("hello".into()).into();
    assert_eq!(error.message.as_deref(), Some("Value hello"));
    LOGS.with(|logs| assert_eq!(*logs.borrow(), ["Value hello"]));
}

#[test]
fn logs_with_compact_options_and_hides_the_message() {
    let error: BaxeError = HiddenLogged::Value("hello".into()).into();
    assert_eq!(error.message, None);
    assert_eq!(error.code, 400);
    assert_eq!(error.error_tag, "hidden");
    LOGS.with(|logs| assert_eq!(*logs.borrow(), ["Secret hello"]));
    let json = serde_json::to_value(error).unwrap();
    assert!(json.get("message").is_none());
}

#[test]
fn hides_without_logging() {
    let error: BaxeError = Hidden::Value("hello".into()).into();
    assert_eq!(error.message, None);
    LOGS.with(|logs| assert!(logs.borrow().is_empty()));
}
