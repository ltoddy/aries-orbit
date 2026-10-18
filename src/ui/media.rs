use std::sync::Arc;

use base64::Engine;
use gpui_kit::{Image, ImageFormat};
use serde_json::Value;

pub fn images(update: &impl serde::Serialize) -> Vec<Arc<Image>> {
    fn collect(value: &Value, images: &mut Vec<Arc<Image>>) {
        match value {
            Value::Object(fields) => {
                if fields.get("type").and_then(Value::as_str) == Some("image") {
                    if let Some(format) = fields
                        .get("mimeType")
                        .and_then(Value::as_str)
                        .and_then(ImageFormat::from_mime_type)
                        && let Some(data) = fields.get("data").and_then(Value::as_str)
                        && let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(data)
                    {
                        images.push(Arc::new(Image::from_bytes(format, bytes)));
                    }
                } else {
                    for key in ["content", "summary"] {
                        if let Some(value) = fields.get(key) {
                            collect(value, images);
                        }
                    }
                }
            }
            Value::Array(values) => {
                for value in values {
                    collect(value, images);
                }
            }
            _ => {}
        }
    }
    let mut images = Vec::new();
    collect(
        &serde_json::to_value(update).expect("ACP notifications serialize"),
        &mut images,
    );
    images
}
