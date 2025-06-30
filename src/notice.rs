use regex::Regex;
use reqwest::Client;
use serde_json::{json, Value};
use anyhow::{anyhow, Result};


pub async fn send(url: &str, user_id: &str, message: &str) {
    let mut frame = template().await;

    let div_at_user = at_user_div(user_id).await;

    let regex = Regex::new("-{3,}").unwrap();

    let messages: Vec<&str> = regex.split(message).collect();

    let details = message_detail(messages).await;

    let hr_element = serde_json::json!({
        "tag": "hr"
    });

    if let Some(arr) = frame["card"]["elements"].as_array_mut() {
        arr.push(div_at_user);

        let len = details.len();
        for (i, d) in details.into_iter().enumerate() {
            arr.push(d);
            if i != len - 1 {
                arr.push(hr_element.clone());
            }
        }
    }

    println!("{}", serde_json::to_string_pretty(&frame).unwrap());

    do_send(url, frame).await.unwrap()
}

async fn do_send(url: &str, data: Value) -> Result<()> {
    let client = Client::new();
    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .json(&data)
        .send()
        .await
        .unwrap();

    if response.status().is_success() {
        Ok(())
    } else {
        Err(anyhow!("Request failed, status: {}", response.status()))
    }
}

async fn template() -> Value {
    json!(
          {
             "msg_type": "interactive",
             "card": {
                 "config": {
                     "wide_screen_mode": true,
                     "enable_forward": false
                 },
                 "elements": [
                 ],
                 "header": {
                     "title": {
                         "content": "🎉 Scheduled Job Fail",
                         "tag": "plain_text"
                     },
                     "template": "blue"
                 }
             }
         }
    )
}

async fn at_user_div(user_id: &str) -> Value {
    json!({
          "tag": "div",
          "text": {
              "content": format!(" <at id={}></at>", user_id),
              "tag": "lark_md"
          }
      }
    )
}

async fn message_detail(details: Vec<&str>) -> Vec<Value> {
    fn detail(detail: &str) -> Value {
        json!({
                "tag": "div",
                "text": {
                    "content": format!("{}", detail),
                    "tag": "lark_md"
                }
            }
        )
    }

    let mut result: Vec<Value> = vec![];

    for d in details {
        let element = detail(d);

        result.push(element);
    }
    result
}

#[cfg(test)]
mod tests {
    use crate::notice;

    #[test]
    fn test_send_normal() {
        let url = "ha?";
        let user_id = "";
        let message = "failed: job1 ---- 07260008601384";
        notice::send(url, user_id, message);
    }

    #[test]
    fn test_send_empty_params() {
        let url = "";
        let user_id = "";
        let message = "";
        notice::send(url, user_id, message);
    }

    // #[test]
    // fn test_dyn() {
    //     trait Anaimal {
    //         fn speak(&self);
    //     }
    //
    //     struct Dog;
    //
    //     impl Anaimal for Dog {
    //         fn speak(&self) {
    //             println!("Wang")
    //         }
    //     }
    //
    //     struct Cat;
    //
    //     impl Anaimal for Cat {
    //         fn speak(&self) {
    //             println!("Miao")
    //         }
    //     }
    //
    //     fn make_speak(a: &dyn Anaimal) {
    //         a.speak()
    //     }
    //
    //
    //     let dog = Dog;
    //     let cat = Cat;
    //
    //     make_speak(&dog);
    //     make_speak(&cat);
    // }
}
