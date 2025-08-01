use serde_json::{json, Value};

pub async fn div_at_user(user_id: &str) -> Value {
    json!({
          "tag": "div",
          "text": {
              "content": format!(" <at id={}></at>", user_id),
              "tag": "lark_md"
          }
      }
    )
}

pub async fn div_project(project: &str) -> Value {
    json!(
           {
                "tag": "div",
                "text": {
                  "content": format!("  in ***{}***", project),
                  "tag": "lark_md"
                }
            }
    )
}

pub async fn div_flow_and_project(flow: &str, project: &str) -> Value {
    json!(
           {
                "tag": "div",
                "text": {
                  "content": format!("    in **{}.{}**",project, flow),
                  "tag": "lark_md"
                }
            }
    )
}

pub async fn div_message(message: &str) -> Value {
    json!(
                  {
                  "tag": "div",
                  "text": {
                    "content": format!("{}", message),
                    "tag": "lark_md"
                  }
                }

    )
}

pub async fn hr() -> Value {
    json!({
        "tag": "hr"
    })
}
