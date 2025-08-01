use crate::bean::Task;
use crate::style;
use crate::style::{div_flow_and_project, div_message, hr};
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;
use style::{div_at_user, div_project};

pub async fn send_with_struct_data(
    url: &str,
    user_id: &str,
    details: &HashMap<String, HashMap<String, Vec<Task>>>,
) -> Result<(), anyhow::Error> {
    let mut frame = template().await;

    if details.is_empty() {
        println!("Nothing to send for ");
        return Ok(());
    }

    let mut elements: Vec<Value> = vec![];

    // @ username
    elements.push(div_at_user(user_id).await);

    for (i, p) in details.into_iter().enumerate() {
        let project = p.0;
        let flows = p.1;

        elements.push(hr().await);
        elements.push(div_project(&project).await);

        for f in flows {
            let flow = f.0;
            let tasks = f.1;

            elements.push(div_flow_and_project(&flow, &project).await);

            let x: Vec<String> = tasks.iter().map(|t| t.to_string().unwrap()).collect();

            let task_message = x.join("\n");

            let message_detail = div_message(task_message.as_str()).await;
            elements.push(message_detail);
        }
    }

    if let Some(arr) = frame["card"]["elements"].as_array_mut() {
        arr.extend(elements);
    }
    do_send(url, frame).await?;

    Ok(())
}

async fn do_send(url: &str, data: Value) -> Result<()> {
    let client = Client::new();
    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .json(&data)
        .send()
        .await
        .map_err(|e| anyhow!("Request failed: {}", e))?;

    let status = &response.status();

    let body = response
        .text()
        .await
        .unwrap_or_else(|_| "Failed to read response body".to_string());

    let connected_and_send_status = match serde_json::from_str::<Value>(&body) {
        Ok(parsed) => {
            if let Some(code) = parsed.get("code").and_then(|v| v.as_i64()) {
                if code != 0 && code != 200 {
                    println!("Get ERROR code : {}", code);
                    println!("Error details: {:?} ", body);
                } else {
                    println!("Get the SUCCESS Code : {}", code)
                }
                code
            } else {
                println!("There is not status code filed {}", body);
                -1
            }
        }
        Err(_) => {
            println!("Parse jons error : {}", body);
            -1
        }
    };

    if !status.is_success() || connected_and_send_status != 0 {
        return Err(anyhow!(
            "Request failed, status: {}, body: {}",
            status,
            body
        ));
    }

    Ok(())
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
    use crate::notice::do_send;
    use serde_json::json;

    #[tokio::test]
    async fn send_demo() -> anyhow::Result<()> {
        let v = json!(
             {
          "card": {
            "config": {
              "enable_forward": false,
              "wide_screen_mode": true
            },
            "header": {
              "template": "red",
              "title": {
                "content": "🚨 任务执行异常通知",
                "tag": "plain_text"
              }
            },
            "elements": [
              {
                "tag": "div",
                "text": {
                  "content": "<at id=ou_7cd06aa3f686d385cae608548dbde217></at> 您好，以下任务执行异常需要处理:",
                  "tag": "lark_md"
                }
              },
              {
                "tag": "div",
                "text": {
                  "content": "it digital",
                  "tag": "lark_md"
                }
              },

                {
                  "tag": "div",
                  "text": {
                    "content": "**任务ID**：dwd_v_income_zy_pdf\n**尝试次数**：1次\n**开始时间**：2025-07-30 01:24:33.879 UTC\n**结束时间**：2025-07-30 01:24:34.029 UTC\n**异常时长**：<font color=\"red\">150ms</font>（极短执行时间可能为资源问题）",
                    "tag": "lark_md"
                  }
                },
                {
                  "tag": "action",
                  "actions": [
                    {
                      "tag": "button",
                      "text": {
                        "content": "查看任务日志",
                        "tag": "plain_text"
                      },
                      "type": "primary",
                      "url": "https://your-task-platform.com/logs/dwd_v_income_zy_pdf"
                    }
                  ]
                },
                {
                  "tag": "hr"
                },
                {
                  "tag": "div",
                  "text": {
                    "content": "**Project Name**: didao\n**任务ID**：dwd_v_income_zy_pdf\n**尝试次数**：1次\n**开始时间**：2025-07-30 01:24:33.879 UTC\n**结束时间**：2025-07-30 01:24:34.029 UTC\n**异常时长**：<font color=\"red\">150ms</font>（极短执行时间可能为资源问题）",
                    "tag": "lark_md"
                  }
                },
                {
                  "tag": "action",
                  "actions": [
                    {
                      "tag": "button",
                      "text": {
                        "content": "查看任务日志",
                        "tag": "plain_text"
                      },
                      "type": "primary",
                      "url": "https://your-task-platform.com/logs/dwd_v_income_zy_pdf"
                    }
                  ]
                }

            ]
          },
          "msg_type": "interactive"
        }
                );
        let url =
            "https://open.feishu.cn/open-apis/bot/v2/hook/6345e240-4325-412a-a96a-c735bdc9b5e2";

        let x = json!({"xiba":"?"});
        // do_send(url, x).await?;
        // do_send(url, v).await?;

        Ok(())
    }

    #[test]
    fn test_send_normal() {
        let url = "ha?";
        let user_id = "";
        let message = "failed: job1 ---- 07260008601384";
        // notice::send(url, user_id, message);
    }

    #[test]
    fn test_send_empty_params() {
        let url = "";
        let user_id = "";
        let message = "";
        // notice::send(url, user_id, message);
    }

    #[tokio::test]
    async fn send_standard_message() -> anyhow::Result<()> {
        let message = json!(
                    {
            "schema": "2.0",
            "config": {
                "update_multi": true,
                "style": {
                    "text_size": {
                        "normal_v2": {
                            "default": "normal",
                            "pc": "normal",
                            "mobile": "heading"
                        }
                    }
                }
            },
            "body": {
                "direction": "vertical",
                "padding": "12px 12px 12px 12px",
                "elements": [
                    {
                        "tag": "markdown",
                        "content": "西湖，位于中国浙江省杭州市西湖区龙井路1号，杭州市区西部，汇水面积为21.22平方千米，湖面面积为6.38平方千米。",
                        "text_align": "left",
                        "text_size": "normal_v2",
                        "margin": "0px 0px 0px 0px"
                    },
                    {
                        "tag": "button",
                        "text": {
                            "tag": "plain_text",
                            "content": "🌞更多景点介绍"
                        },
                        "type": "default",
                        "width": "default",
                        "size": "medium",
                        "behaviors": [
                            {
                                "type": "open_url",
                                "default_url": "https://baike.baidu.com/item/%E8%A5%BF%E6%B9%96/4668821",
                                "pc_url": "",
                                "ios_url": "",
                                "android_url": ""
                            }
                        ],
                        "margin": "0px 0px 0px 0px"
                    }
                ]
            },
            "header": {
                "title": {
                    "tag": "plain_text",
                    "content": "今日旅游推荐"
                },
                "subtitle": {
                    "tag": "plain_text",
                    "content": ""
                },
                "template": "blue",
                "padding": "12px 12px 12px 12px"
            }
        }
                );

        let url =
            "https://open.feishu.cn/open-apis/bot/v2/hook/6345e240-4325-412a-a96a-c735bdc9b5e2";
        do_send(url, message).await?;

        Ok(())
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
