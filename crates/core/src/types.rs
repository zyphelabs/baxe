///
/// Macro that creates the  `BaxeError` type.
/// 
/// # Examples
/// 
/// ```
/// use baxe::baxe_error;
/// # type Tags = String;
/// baxe_error!(Tags, serde(rename_all = "camelCase"), derive(Clone));
/// ```
///
/// ```
/// use baxe::baxe_error;
/// baxe_error!(String, serde(rename_all = "camelCase"));
/// ```
///
/// ```
/// use baxe::baxe_error;
/// baxe_error!(String,);
/// ```
#[macro_export]
macro_rules! baxe_error {
    ( $error_tag_ty:ty, $($extra_attr:meta),* ) => {
        #[derive(std::fmt::Debug, serde::Serialize)]
        $(#[$extra_attr])*

        pub struct BaxeError {
            #[serde(skip)]
            pub status_code: axum::http::StatusCode,
            #[serde(skip_serializing_if = "Option::is_none")]
            pub message: Option<String>,
            pub code: u16,
            pub error_tag: $error_tag_ty,
        }
        
        impl std::fmt::Display for BaxeError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{:?}", self.message)
            }
        }
        
        impl std::error::Error for BaxeError {}
        
        impl axum::response::IntoResponse for BaxeError {
            fn into_response(self) -> axum::response::Response {
                (self.status_code, axum::Json(self)).into_response()
            }
        }

        impl BaxeError {
            pub fn new(status_code: axum::http::StatusCode, message: Option<String>, code: u16, error_tag: String) -> Self {
                use std::str::FromStr;
                Self {
                    status_code,
                    message,
                    code,
                    error_tag: <$error_tag_ty>::from_str(&error_tag).unwrap_or_default()
                }
            }
        }
    };
}
