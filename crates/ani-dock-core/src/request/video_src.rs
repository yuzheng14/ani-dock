use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoSrc {
    pub(crate) deviceid: String,
    pub(crate) src_use_cases: Vec<SrcUseCase>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SrcUseCase {
    pub(crate) device_type: u32,
    pub(crate) src: Src,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Src {
    pub(crate) playlist: String,
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use crate::request::common::CommonResponseBody;

    use super::*;

    #[test]
    fn deserializes_current_api_response() -> Result<(), Box<dyn Error>> {
        let response = r#"
            {
                "data": {
                    "deviceid": "<deviceid>",
                    "srcUseCases": [
                        {
                            "deviceType": 1,
                            "src": {
                                "playlist": "<playlist url>"
                            }
                        }
                    ]
                }
            }
        "#;

        let CommonResponseBody::Data(video_src) =
            serde_json::from_str::<CommonResponseBody<VideoSrc, String>>(response)?
        else {
            panic!("expected a successful video source response");
        };

        assert_eq!(video_src.deviceid, "<deviceid>");
        assert_eq!(video_src.src_use_cases.len(), 1);
        assert_eq!(video_src.src_use_cases[0].device_type, 1);
        assert_eq!(video_src.src_use_cases[0].src.playlist, "<playlist url>");

        Ok(())
    }
}
