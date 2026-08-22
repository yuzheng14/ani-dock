use std::fmt::Display;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiError {
    code: u32,
    message: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    details: Vec<Value>,
}

impl Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[code]: {}, [message]: {}", self.code, self.message)?;

        if !self.status.is_empty() {
            write!(f, ", [status]: {}", self.status)?;
        }
        if !self.details.is_empty() {
            write!(f, ", [details]: {:?}", self.details)?;
        }

        Ok(())
    }
}

impl std::error::Error for ApiError {}

/// common response body of bahmut's api returned json
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CommonResponseBody<D, E> {
    Data(D),
    Error(E),
}

impl<D, E> CommonResponseBody<D, E> {
    pub fn into_result(self) -> Result<D, E> {
        match self {
            Self::Data(data) => Ok(data),
            Self::Error(error) => Err(error),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DirectDataResponseBody<D, E> {
    Data(D),
    Error { error: E },
}

impl<D, E> DirectDataResponseBody<D, E> {
    pub fn into_result(self) -> Result<D, E> {
        match self {
            Self::Data(data) => Ok(data),
            Self::Error { error } => Err(error),
        }
    }
}

#[cfg(test)]
mod test {
    use std::error::Error;

    use crate::request::token::{Token, TokenError};

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct TestData {
        data_string: String,
    }

    #[test]
    fn common_response_body_serde() -> Result<(), Box<dyn Error>> {
        let data_json = r#"{"data":{"dataString":"data_string"}}"#;
        let error_json = r#"{"error":"error_string"}"#;
        let data = CommonResponseBody::<TestData, String>::Data(TestData {
            data_string: "data_string".to_string(),
        });
        let error = CommonResponseBody::<TestData, String>::Error("error_string".to_string());

        assert_eq!(
            serde_json::from_str::<CommonResponseBody<TestData, String>>(data_json)?,
            data
        );
        assert_eq!(
            serde_json::from_str::<CommonResponseBody<TestData, String>>(error_json)?,
            error
        );
        assert_eq!(serde_json::to_string(&data)?, data_json);
        assert_eq!(serde_json::to_string(&error)?, error_json);

        Ok(())
    }

    #[test]
    fn common_response_body_converts_into_result() {
        let data = TestData {
            data_string: "data_string".to_string(),
        };

        assert_eq!(
            CommonResponseBody::<TestData, String>::Data(data.clone()).into_result(),
            Ok(data)
        );
        assert_eq!(
            CommonResponseBody::<TestData, String>::Error("error_string".to_string()).into_result(),
            Err("error_string".to_string())
        );
    }

    #[test]
    fn common_response_body_deserializes_structured_api_error() -> Result<(), Box<dyn Error>> {
        let response = r#"{
            "error": {
                "code": 1007,
                "message": "裝置驗證異常！",
                "status": "",
                "details": []
            }
        }"#;

        let CommonResponseBody::<TestData, ApiError>::Error(error) =
            serde_json::from_str(response)?
        else {
            panic!("expected an API error response");
        };

        assert_eq!(error.code, 1007);
        assert_eq!(error.message, "裝置驗證異常！");
        assert_eq!(error.status, "");
        assert!(error.details.is_empty());
        assert_eq!(error.to_string(), "[code]: 1007, [message]: 裝置驗證異常！");

        Ok(())
    }

    #[test]
    fn direct_data_response_body_serde() -> Result<(), Box<dyn Error>> {
        let data_json = r#"{"dataString":"data_string"}"#;
        let error_json = r#"{"error":"error_string"}"#;
        let data = DirectDataResponseBody::<TestData, String>::Data(TestData {
            data_string: "data_string".to_string(),
        });
        let error = DirectDataResponseBody::<TestData, String>::Error {
            error: "error_string".to_string(),
        };

        assert_eq!(
            serde_json::from_str::<DirectDataResponseBody<TestData, String>>(data_json)?,
            data
        );
        assert_eq!(
            serde_json::from_str::<DirectDataResponseBody<TestData, String>>(error_json)?,
            error
        );
        assert_eq!(serde_json::to_string(&data)?, data_json);
        assert_eq!(serde_json::to_string(&error)?, error_json);

        Ok(())
    }

    #[test]
    fn direct_data_response_body_converts_into_result() {
        let data = TestData {
            data_string: "data_string".to_string(),
        };

        assert_eq!(
            DirectDataResponseBody::<TestData, String>::Data(data.clone()).into_result(),
            Ok(data)
        );
        assert_eq!(
            DirectDataResponseBody::<TestData, String>::Error {
                error: "error_string".to_string(),
            }
            .into_result(),
            Err("error_string".to_string())
        );
    }

    #[test]
    fn direct_data_response_body_with_true_data() -> Result<(), Box<dyn Error>> {
        let content =
            r#"{"src":"","animeSn":59221,"r18":3,"vip":false,"time":0,"login":false,"promote":[]}"#;

        assert_eq!(
            DirectDataResponseBody::<Token, TokenError>::Data(Token {
                src: String::from(""),
                anime_sn: 59221,
                r18: 3,
                vip: false,
                time: 0,
                login: false,
            }),
            serde_json::from_str(content)?
        );

        Ok(())
    }
}
