use agent_client_protocol::schema::v1::SessionUpdate;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateDisplay {
    title: String,
    body: String,
}

impl UpdateDisplay {
    pub fn into_parts(self) -> (String, String) {
        (self.title, self.body)
    }
}

pub fn code_block(language: &str, text: &str) -> String {
    let longest = text.split(|c| c != '`').map(str::len).max().unwrap_or(0);
    let fence = "`".repeat(longest.max(2) + 1);
    format!("{fence}{language}\n{text}\n{fence}")
}

fn json(value: &Value) -> String {
    code_block("json", &format!("{value:#}"))
}

fn string<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}

fn status(value: &Value) -> &str {
    match value.as_str().unwrap_or_default() {
        "pending" => "等待中",
        "in_progress" => "进行中",
        "completed" => "已完成",
        "failed" => "失败",
        "cancelled" => "已取消",
        other => other,
    }
}

fn content(value: &Value) -> String {
    match string(value, "type") {
        "text" => string(value, "text").to_owned(),
        "image" | "audio" => format!(
            "{} · {}",
            if string(value, "type") == "image" { "图片" } else { "音频" },
            string(value, "mimeType")
        ),
        "resource_link" => format!(
            "资源：{}\n\n{}\n\n{}",
            string(value, "name"),
            code_block("", string(value, "uri")),
            string(value, "description")
        ),
        "resource" => {
            let resource = &value["resource"];
            format!(
                "资源内容\n\n{}\n\n{}",
                code_block("", string(resource, "uri")),
                if let Some(text) = resource["text"].as_str() {
                    code_block("", text)
                } else {
                    json(resource)
                }
            )
        }
        _ => json(value),
    }
}

fn tool_content(value: &Value) -> String {
    match string(value, "type") {
        "content" => content(&value["content"]),
        "diff" => format!(
            "文件修改\n\n{}\n\n修改前\n\n{}\n\n修改后\n\n{}",
            code_block("", string(value, "path")),
            code_block("", value["oldText"].as_str().unwrap_or("（新建文件）")),
            code_block("", string(value, "newText"))
        ),
        "terminal" => format!("终端引用\n\n{}", code_block("", string(value, "terminalId"))),
        _ => json(value),
    }
}

impl From<&SessionUpdate> for UpdateDisplay {
    fn from(update: &SessionUpdate) -> Self {
        let value = match serde_json::to_value(update) {
            Ok(value) => value,
            Err(error) => {
                return Self {
                    title: "会话更新".into(),
                    body: format!("无法解析会话更新：{error}\n\n{update:#?}"),
                };
            }
        };
        let kind = string(&value, "sessionUpdate");
        let (title, body) = match kind {
            "agent_message_chunk" | "user_message_chunk" | "agent_thought_chunk" => (
                match kind {
                    "user_message_chunk" => "用户内容",
                    "agent_thought_chunk" => "思考内容",
                    _ => "Aries 内容",
                }
                .to_owned(),
                content(&value["content"]),
            ),
            "tool_call" | "tool_call_update" => {
                let name = string(&value, "title");
                let id = string(&value, "toolCallId");
                let title = if name.is_empty() {
                    format!("工具更新 · {id}")
                } else {
                    format!("工具调用 · {name}")
                };
                let mut parts = vec![format!("调用 ID：{id}")];
                if value.get("status").is_some() {
                    parts.push(format!("状态：{}", status(&value["status"])));
                }
                for (key, label) in [("name", "工具"), ("kind", "类型")] {
                    if let Some(field) = value.get(key).and_then(Value::as_str) {
                        parts.push(format!("{label}：{field}"));
                    }
                }
                for (key, label) in
                    [("rawInput", "输入参数"), ("rawOutput", "输出结果"), ("locations", "文件位置")]
                {
                    if let Some(field) = value.get(key) {
                        parts.push(format!("{label}\n\n{}", json(field)));
                    }
                }
                if let Some(contents) = value["content"].as_array() {
                    parts.extend(contents.iter().map(tool_content));
                }
                (title, parts.join("\n\n"))
            }
            "plan" => (
                "执行计划".into(),
                value["entries"]
                    .as_array()
                    .map(|entries| {
                        entries
                            .iter()
                            .map(|entry| {
                                format!(
                                    "- {} · {} · {}",
                                    status(&entry["status"]),
                                    string(entry, "priority"),
                                    string(entry, "content")
                                )
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .filter(|text| !text.is_empty())
                    .unwrap_or_else(|| "计划已清空".into()),
            ),
            "current_mode_update" => {
                ("会话模式".into(), code_block("", string(&value, "currentModeId")))
            }
            "config_option_update" => ("配置选项".into(), json(&value["configOptions"])),
            "usage_update" => (
                "用量".into(),
                format!(
                    "上下文：{} / {} tokens{}",
                    value["used"],
                    value["size"],
                    value
                        .get("cost")
                        .map(|cost| format!(
                            "\n\n费用：{} {}",
                            cost["amount"],
                            string(cost, "currency")
                        ))
                        .unwrap_or_default()
                ),
            ),
            "notice" => (
                format!("通知 · {}", string(&value, "title")),
                format!("{}\n\n{}", string(&value, "severity"), string(&value, "description")),
            ),
            "compaction_update" => {
                let mut parts = vec![format!(
                    "{} · {}",
                    string(&value, "compactionId"),
                    status(&value["status"])
                )];
                if let Some(summary) = value["summary"].as_array() {
                    parts.extend(summary.iter().map(content));
                }
                if let Some(error) = value.get("error").filter(|error| !error.is_null()) {
                    parts.push(json(error));
                }
                ("上下文压缩".into(), parts.join("\n\n"))
            }
            "compaction_summary_chunk" => (
                "压缩摘要".into(),
                format!("{}\n\n{}", string(&value, "compactionId"), content(&value["content"])),
            ),
            _ => (format!("会话更新 · {kind}"), json(&value)),
        };
        UpdateDisplay { title, body }
    }
}
